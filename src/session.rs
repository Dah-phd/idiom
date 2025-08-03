use crate::configs::{FileType, APP_FOLDER};
use crate::error::{IdiomError, IdiomResult};
use crate::global_state::GlobalState;
use crate::workspace::Workspace;
use dirs::data_local_dir;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DATA_FILE: &str = "data.json";
const META_FILE: &str = "meta.json";

#[derive(Serialize)]
struct StoreFileData<'a> {
    path: PathBuf,
    file_type: FileType,
    content: Option<Vec<&'a str>>,
}

#[derive(Deserialize)]
struct LoadedFileData {
    path: PathBuf,
    file_type: FileType,
    content: Option<Vec<String>>,
}

// enough to restore last session
#[derive(Deserialize, Serialize)]
struct MetaData {
    path: PathBuf,
}

pub fn store_session(ws: &mut Workspace, max_sessions: usize) -> bool {
    if ws.is_empty() {
        return true;
    }

    // get app folder
    let Some(mut store) = data_local_dir() else {
        return false;
    };
    store.push(APP_FOLDER);

    // create app folded if not exists
    if !store.exists() && std::fs::create_dir(&store).is_err() {
        return false;
    }

    let Ok(cwd) = std::env::current_dir() else { return false };
    let cwd_hash = hash_path(&cwd);

    // cleanup
    clean_up(&store, cwd_hash, max_sessions);

    let Ok(epoch) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return false;
    };
    let timestamp = epoch.as_secs();

    // create session folder
    let folder_name = format!("{timestamp}_{cwd_hash}");
    store.push(folder_name);
    if std::fs::create_dir(&store).is_err() {
        return false;
    }

    let md = MetaData { path: cwd };
    let Ok(md_contents) = serde_json::to_string(&md) else {
        return false;
    };
    let mut md_path = store.clone();
    md_path.push(META_FILE);
    let Ok(..) = std::fs::write(md_path, md_contents) else {
        return false;
    };

    let mut session_files = vec![];
    for editor in ws.iter() {
        let content = if editor.is_saved().unwrap_or(true) {
            None
        } else {
            Some(editor.content.iter().map(|l| l.content.as_str()).collect())
        };
        session_files.push(StoreFileData { content, file_type: editor.file_type, path: editor.path.clone() })
    }

    let Ok(session_contents) = serde_json::to_string(&session_files) else {
        return false;
    };

    store.push(DATA_FILE);
    std::fs::write(store, session_contents).is_ok()
}

pub fn restore_last_sesson() -> IdiomResult<PathBuf> {
    let mut store = data_local_dir().ok_or(IdiomError::io_not_found("Unable to determine session storage"))?;
    store.push(APP_FOLDER);

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

pub async fn load_session(ws: &mut Workspace, gs: &mut GlobalState) {
    let Some(mut store) = data_local_dir() else { return };
    store.push(APP_FOLDER);

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
            if let Err(error) = ws.new_from_session(fd.path, fd.file_type, fd.content, gs).await {
                gs.error(error);
            }
        }
        _ = std::fs::remove_dir_all(path);
        gs.insert_mode();
        return;
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
    use super::{LoadedFileData, StoreFileData};
    use crate::configs::FileType;
    use std::path::PathBuf;

    #[test]
    fn separate_serde() {
        let serialized = vec![StoreFileData {
            path: PathBuf::from("/home/test"),
            file_type: FileType::Rust,
            content: Some(vec!["text", "more text"]),
        }];

        let as_txt = serde_json::to_string(&serialized).unwrap();

        let deserialized: Vec<LoadedFileData> = serde_json::from_str(&as_txt).unwrap();

        assert_eq!(serialized.len(), deserialized.len());
        assert_eq!(serialized[0].path, deserialized[0].path);
        assert_eq!(serialized[0].file_type, deserialized[0].file_type);
        assert_eq!(serialized[0].content.as_ref().unwrap(), deserialized[0].content.as_ref().unwrap());
    }
}
