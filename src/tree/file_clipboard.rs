use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, IdiomEvent},
    popups::PopupSelector,
};

#[derive(Default, Debug)]
enum ActionType {
    Cut,
    Copy,
    #[default]
    None,
}

#[derive(Default, Debug)]
pub struct FileClipboard {
    marked_paths: Vec<PathBuf>,
    action_type: ActionType,
}

impl FileClipboard {
    pub fn clear(&mut self) {
        self.marked_paths.clear();
        self.action_type = ActionType::None;
    }

    pub fn get_mark(&self, path: &Path) -> Option<&str> {
        match self.action_type {
            ActionType::None => None,
            ActionType::Copy => match self.check_if_selected(path) {
                true => Some(" c "),
                false => None,
            },
            ActionType::Cut => match self.check_if_selected(path) {
                true => Some(" x "),
                false => None,
            },
        }
    }

    pub fn cut(&mut self, path_new: PathBuf) {
        match self.action_type {
            ActionType::Cut => {
                self.toggle(path_new);
            }
            ActionType::Copy => {
                self.marked_paths.clear();
                self.action_type = ActionType::Cut;
                self.marked_paths.push(path_new);
            }
            ActionType::None => {
                self.action_type = ActionType::Cut;
                self.marked_paths.push(path_new);
            }
        }
    }

    pub fn force_cut(&mut self, path: PathBuf) {
        self.marked_paths.clear();
        self.marked_paths.push(path);
        self.action_type = ActionType::Cut;
    }

    pub fn copy(&mut self, new_path: PathBuf) {
        match self.action_type {
            ActionType::Copy => {
                self.toggle(new_path);
            }
            ActionType::Cut => {
                self.marked_paths.clear();
                self.action_type = ActionType::Copy;
                self.marked_paths.push(new_path);
            }
            ActionType::None => {
                self.action_type = ActionType::Copy;
                self.marked_paths.push(new_path);
            }
        }
    }

    pub fn force_copy(&mut self, path: PathBuf) {
        self.marked_paths.clear();
        self.marked_paths.push(path);
        self.action_type = ActionType::Copy;
    }

    pub fn paste<P: AsRef<Path>>(&mut self, selected_path: P, gs: &mut GlobalState) {
        match std::mem::take(&mut self.action_type) {
            ActionType::None => (),
            ActionType::Copy => {
                let mut error_list = Vec::new();
                let mut dest = PathBuf::new();
                dest.push(selected_path);
                for path in self.marked_paths.drain(..) {
                    fs_recursive_copy(dest.clone(), path, &mut error_list);
                }
                if !error_list.is_empty() {
                    gs.popup(PopupSelector::message_list(error_list));
                }
            }
            ActionType::Cut => {
                let mut error_list = Vec::new();
                let mut dest = PathBuf::new();
                dest.push(selected_path);
                for from_path in self.marked_paths.drain(..) {
                    match fs_move(dest.clone(), from_path.clone()) {
                        Ok(to_path) => gs.event.push(IdiomEvent::RenamedFile { from_path, to_path }),
                        Err(error) => error_list.push(error),
                    };
                }
                if !error_list.is_empty() {
                    gs.popup(PopupSelector::message_list(error_list));
                }
            }
        }
    }

    fn toggle(&mut self, new: PathBuf) {
        for (idx, selected_path) in self.marked_paths.iter().enumerate() {
            if selected_path == &new {
                self.marked_paths.remove(idx);
                return;
            }
            if new.starts_with(selected_path) {
                return;
            }
        }
        self.marked_paths.push(new);
    }

    fn check_if_selected(&self, path: &Path) -> bool {
        for selected_path in self.marked_paths.iter() {
            if path.starts_with(selected_path) {
                return true;
            }
        }
        false
    }
}

fn fs_recursive_copy(mut dest: PathBuf, path: PathBuf, error_list: &mut Vec<IdiomError>) {
    if path.is_dir() {
        match path.file_name() {
            Some(name) => dest.push(name),
            None => {
                error_list.push(IdiomError::io_other(format!("Unable to determine path stem: {path:?}")));
                return;
            }
        }
        let dir_content = match std::fs::read_dir(path) {
            Ok(dir) => dir,
            Err(error) => {
                error_list.push(IdiomError::IOError(error));
                return;
            }
        };
        for entry_result in dir_content {
            match entry_result {
                Ok(entry) => fs_recursive_copy(dest.to_owned(), entry.path(), error_list),
                Err(error) => error_list.push(IdiomError::IOError(error)),
            }
        }
    } else {
        match path.file_stem() {
            Some(name) => {
                if !dest.exists() {
                    if let Err(error) = std::fs::create_dir_all(dest.as_path()) {
                        error_list.push(IdiomError::IOError(error));
                        return;
                    }
                }
                let mut suffixed_name = SuffixedNameIter::new(name, path.extension());
                let mut potential_dest = dest.join(suffixed_name.next().unwrap());
                while potential_dest.exists() {
                    potential_dest = dest.join(suffixed_name.next().unwrap());
                }
                if let Err(error) = std::fs::copy(path, potential_dest) {
                    error_list.push(IdiomError::IOError(error));
                };
            }
            None => {
                error_list.push(IdiomError::io_other(format!("Unable to determine path stem: {path:?}")));
            }
        }
    }
}

/// dest should be a dir
fn fs_move(mut dest: PathBuf, path: PathBuf) -> IdiomResult<PathBuf> {
    let name = path.file_name().ok_or(IdiomError::io_other(format!("Unable to determine path stem: {path:?}")))?;
    dest.push(name);
    if dest.exists() {
        return Err(IdiomError::io_exists(format!("Expected new path already exists: {dest:?}")));
    }
    std::fs::rename(path, &dest)?;
    Ok(dest)
}

struct SuffixedNameIter<'a> {
    count: usize,
    name: &'a OsStr,
    extension: Option<&'a OsStr>,
}

impl<'a> SuffixedNameIter<'a> {
    fn new(name: &'a OsStr, extension: Option<&'a OsStr>) -> Self {
        Self { count: 0, name, extension }
    }
}

impl Iterator for SuffixedNameIter<'_> {
    type Item = OsString;

    fn next(&mut self) -> Option<Self::Item> {
        let mut new_name = match self.count {
            0 => self.name.to_owned(),
            idx => {
                let mut new_name = self.name.to_owned();
                new_name.push("_copy");
                new_name.push(idx.to_string());
                new_name
            }
        };
        self.count += 1;
        if let Some(extension) = self.extension {
            new_name.push(".");
            new_name.push(extension);
        }
        Some(new_name)
    }
}
