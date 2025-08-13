use crate::configs::{FileType, APP_FOLDER};
use crate::error::{IdiomError, IdiomResult};
use crate::global_state::{GlobalState, IdiomEvent};
use crate::workspace::{cursor::Cursor, Workspace};
use dirs::data_local_dir;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DATA_FILE: &str = "data.json";
const META_FILE: &str = "meta.json";

#[derive(Serialize, Debug)]
struct StoreFileData<'a> {
    path: PathBuf,
    file_type: FileType,
    content: Option<Vec<&'a str>>,
    cursor: Cursor,
}

impl<'a> StoreFileData<'a> {
    fn from_workspace(ws: &'a Workspace) -> Vec<Self> {
        ws.iter()
            .map(|editor| {
                let store_content = !editor.is_saved().unwrap_or_default();
                let content = store_content.then_some(editor.content.iter().map(|l| l.content.as_str()).collect());
                StoreFileData {
                    content,
                    file_type: editor.file_type,
                    path: editor.path.clone(),
                    cursor: editor.cursor.clone(),
                }
            })
            .collect()
    }
}

#[derive(Deserialize)]
struct LoadedFileData {
    path: PathBuf,
    file_type: FileType,
    content: Option<Vec<String>>,
    cursor: Cursor,
}

// enough to restore last session
#[derive(Deserialize, Serialize)]
struct MetaData {
    path: PathBuf,
}

#[derive(Default)]
pub enum SessionStatus {
    #[default]
    Failed,
    FailedNoUnsaved,
    Stored,
}

#[inline]
pub fn store_session(ws: &Workspace, max_sessions: usize) -> SessionStatus {
    // get app folder
    let Some(store) = get_store_path() else {
        return SessionStatus::Failed;
    };
    // create app folded if not exists
    if !store.exists() && std::fs::create_dir(&store).is_err() {
        return SessionStatus::Failed;
    }
    create_and_store_session(store, ws, max_sessions)
}

// temp dir testible
fn create_and_store_session(mut store: PathBuf, ws: &Workspace, max_sessions: usize) -> SessionStatus {
    let Ok(cwd) = std::env::current_dir() else {
        return SessionStatus::Failed;
    };
    let cwd_hash = hash_path(&cwd);

    // cleanup
    clean_up(&store, cwd_hash, max_sessions);

    let Ok(epoch) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return SessionStatus::Failed;
    };
    let timestamp = epoch.as_secs();

    // create session folder
    let folder_name = format!("{timestamp}_{cwd_hash}");
    store.push(folder_name);
    if std::fs::create_dir(&store).is_err() {
        return SessionStatus::Failed;
    }

    let md = MetaData { path: cwd };
    let Ok(md_contents) = serde_json::to_string(&md) else {
        return SessionStatus::Failed;
    };
    let mut md_path = store.clone();
    md_path.push(META_FILE);
    let Ok(..) = std::fs::write(md_path, md_contents) else {
        return SessionStatus::Failed;
    };

    let session_files = StoreFileData::from_workspace(ws);

    let Ok(session_contents) = serde_json::to_string(&session_files) else {
        if session_files.iter().any(|fd| fd.content.is_some()) {
            return SessionStatus::Failed;
        };
        return SessionStatus::FailedNoUnsaved;
    };

    store.push(DATA_FILE);
    match std::fs::write(store, session_contents) {
        Ok(..) => SessionStatus::Stored,
        Err(..) => {
            if session_files.iter().any(|fd| fd.content.is_some()) {
                SessionStatus::Failed
            } else {
                SessionStatus::FailedNoUnsaved
            }
        }
    }
}

#[inline]
pub fn restore_last_sesson() -> IdiomResult<PathBuf> {
    let store = get_store_path().ok_or(IdiomError::io_not_found("Unable to determine session storage"))?;
    read_last_session_working_dir(store)
}

// temp dir testible
fn read_last_session_working_dir(store: PathBuf) -> IdiomResult<PathBuf> {
    let mut last_session_path = std::fs::read_dir(&store)?
        .flatten()
        .map(|dir_entry| {
            let path = dir_entry.path();
            (get_timestamp(&path), path)
        })
        .max_by(|(ts, _), (r_ts, _)| ts.cmp(r_ts))
        .ok_or(IdiomError::io_not_found("Unable to find last recorded session!"))?
        .1;
    last_session_path.push(META_FILE);
    let md_str = std::fs::read_to_string(last_session_path)?;
    let md: MetaData = serde_json::from_str(&md_str)
        .map_err(|_| IdiomError::GeneralError("Failed to parse last session metadata!".to_owned()))?;
    Ok(md.path)
}

#[inline]
pub async fn load_session(ws: &mut Workspace, gs: &mut GlobalState) {
    if let Some(store) = get_store_path() {
        load_session_if_exists(store, ws, gs).await
    }
}

// temp dir testible
async fn load_session_if_exists(store: PathBuf, ws: &mut Workspace, gs: &mut GlobalState) {
    if !store.exists() {
        return;
    };

    let Ok(cwd) = std::env::current_dir() else { return };
    let cwd_hash = hash_path(&cwd);

    let Ok(dir_data) = std::fs::read_dir(&store) else {
        return;
    };

    for dir in dir_data.flatten() {
        let path = dir.path();
        if !path.is_dir() {
            continue;
        };

        if !hash_match_stored(cwd_hash, &path) {
            continue;
        }

        let mut data_path = path.clone();
        data_path.push(DATA_FILE);

        let Ok(data) = std::fs::read_to_string(data_path) else {
            return;
        };
        let Ok(session) = serde_json::from_str::<Vec<LoadedFileData>>(&data) else {
            return;
        };

        for fd in session.into_iter().rev() {
            if let Err(error) = ws.new_from_session(fd.path, fd.file_type, fd.cursor, fd.content, gs).await {
                gs.error(error);
            }
        }
        _ = std::fs::remove_dir_all(path);

        let Some(editor) = ws.get_active() else { return };
        gs.event.push(IdiomEvent::SelectPath(editor.path.to_owned()));

        if gs.is_select() {
            gs.insert_mode();
        };
    }
}

fn clean_up(store: &Path, cwd_hash: u64, max_sessions: usize) {
    let Ok(dir_data) = std::fs::read_dir(store) else {
        return;
    };
    let mut paths = dir_data
        .flatten()
        .filter_map(|dir_entry| {
            let path = dir_entry.path();
            let (ts, hash_val) = split_path(&path)?;
            Some((ts, hash_val, path))
        })
        .collect::<Vec<(u64, u64, PathBuf)>>();

    if paths.len() > max_sessions {
        // biggest to smallest (bigger is newer)
        paths.sort_by(|(ts, ..), (r_ts, ..)| r_ts.cmp(ts));
        while paths.len() > max_sessions {
            _ = std::fs::remove_dir_all(paths.remove(max_sessions).2);
        }
    };

    for (_, hash_val, dir) in paths {
        if hash_val != cwd_hash {
            continue;
        };
        _ = std::fs::remove_dir_all(dir);
    }
}

fn get_store_path() -> Option<PathBuf> {
    let mut store = data_local_dir()?;
    store.push(APP_FOLDER);
    Some(store)
}

// (timestamp, hash)
fn split_path(path: &Path) -> Option<(u64, u64)> {
    let full = path.file_stem()?.to_string_lossy();
    let (ts, ph) = full.split_once('_')?;
    Some((ts.parse().ok()?, ph.parse().ok()?))
}

fn hash_match_stored(hash: u64, path: &Path) -> bool {
    let Some(stem) = path.file_stem() else { return false };
    let stem_str = stem.to_string_lossy();
    let Some((_, hash_str)) = stem_str.split_once('_') else {
        return false;
    };
    let Ok(hash_val) = hash_str.parse::<u64>() else {
        return false;
    };
    hash_val == hash
}

fn get_timestamp(path: &Path) -> Option<u64> {
    let full = path.file_stem()?.to_string_lossy();
    full.split('_').next().and_then(|ts| ts.parse().ok())
}

fn hash_path(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{
        create_and_store_session, load_session_if_exists, read_last_session_working_dir, LoadedFileData, SessionStatus,
        StoreFileData,
    };
    use crate::configs::FileType;
    use crate::ext_tui::CrossTerm;
    use crate::global_state::GlobalState;
    use crate::utils::test::TempDir;
    use crate::workspace::{
        cursor::Cursor,
        tests::{mock_ws, mock_ws_empty},
    };
    use idiom_tui::{layout::Rect, Backend};
    use std::path::PathBuf;

    #[tokio::test]
    async fn store_and_load() {
        let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());
        let mut ws = mock_ws(vec![String::from("test data"), String::from("second line")]);
        assert_eq!(ws.get_active().unwrap().path, PathBuf::from("test-path"));
        assert!(!StoreFileData::from_workspace(&ws).is_empty());
        let mut receiver_ws = mock_ws_empty();
        assert!(receiver_ws.is_empty());
        let temp_dir = TempDir::new("session-store").unwrap();
        // store session
        let status = create_and_store_session(temp_dir.path().to_owned(), &ws, 10);
        assert!(matches!(status, SessionStatus::Stored));
        // check if can be mapped to last session
        assert!(read_last_session_working_dir(temp_dir.path().to_owned()).is_ok());
        // loading session
        load_session_if_exists(temp_dir.path().to_owned(), &mut receiver_ws, &mut gs).await;
        assert!(!receiver_ws.is_empty());
        // confirm stored is same as loaded
        let expected_content = ws.get_active().unwrap().content.iter().map(|l| l.content.as_str()).collect::<Vec<_>>();
        let content = receiver_ws.get_active().unwrap().content.iter().map(|l| l.content.as_str()).collect::<Vec<_>>();
        assert_eq!(content, expected_content);
    }

    #[test]
    fn separate_serde() {
        let serialized = vec![StoreFileData {
            path: PathBuf::from("/home/test"),
            file_type: FileType::Rust,
            content: Some(vec!["text", "more text"]),
            cursor: Cursor::default(),
        }];

        let as_txt = serde_json::to_string(&serialized).unwrap();

        let deserialized: Vec<LoadedFileData> = serde_json::from_str(&as_txt).unwrap();

        assert_eq!(serialized.len(), deserialized.len());
        assert_eq!(serialized[0].path, deserialized[0].path);
        assert_eq!(serialized[0].file_type, deserialized[0].file_type);
        assert_eq!(serialized[0].content.as_ref().unwrap(), deserialized[0].content.as_ref().unwrap());
    }
}
