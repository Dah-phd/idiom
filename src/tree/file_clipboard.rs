use std::path::{Path, PathBuf};

use crate::error::{IdiomError, IdiomResult};

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

    pub fn get_mark(&self, path: &PathBuf) -> Option<&str> {
        match self.action_type {
            ActionType::None => None,
            ActionType::Copy => match check_if_selected(&self.marked_paths, path) {
                true => Some(" c "),
                false => None,
            },
            ActionType::Cut => match check_if_selected(&self.marked_paths, path) {
                true => Some(" x "),
                false => None,
            },
        }
    }

    pub fn cut(&mut self, path_new: PathBuf) {
        match self.action_type {
            ActionType::Cut => {
                toggle(&mut self.marked_paths, path_new);
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
                toggle(&mut self.marked_paths, new_path);
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

    pub fn paste<P: AsRef<Path>>(&mut self, selected_path: P) {
        match std::mem::take(&mut self.action_type) {
            ActionType::None => return,
            ActionType::Copy => {
                let mut dest = PathBuf::new();
                dest.push(selected_path);
                for path in self.marked_paths.drain(..) {
                    perform_copy(dest.clone(), path);
                }
            }
            ActionType::Cut => {
                let mut dest = PathBuf::new();
                dest.push(selected_path);
                for path in self.marked_paths.drain(..) {
                    perform_copy(dest.clone(), path);
                }
            }
        }
    }
}

fn perform_copy(mut dest: PathBuf, path: PathBuf) -> IdiomResult<()> {
    let name = path.file_name().ok_or(IdiomError::io_other(format!("Unable to determine file stem: {path:?}")))?;
    dest.push(name);
    panic!("Expected: {dest:?}");
    Ok(())
}

fn perform_cut(mut dest: PathBuf, path: PathBuf) -> IdiomResult<PathBuf> {
    let name = path.file_name().ok_or(IdiomError::io_other(format!("Unable to determine file stem: {path:?}")))?;
    dest.push(name);
    if dest.exists() {
        return Err(IdiomError::io_exists(format!("Expected new path already exists: {dest:?}")));
    }
    std::fs::rename(path, &dest)?;
    Ok(dest)
}

fn toggle(dest: &mut Vec<PathBuf>, new: PathBuf) {
    for (idx, selected_path) in dest.iter().enumerate() {
        if selected_path == &new {
            dest.remove(idx);
            return;
        }
        if new.starts_with(selected_path) {
            return;
        }
    }
    dest.push(new);
}

fn check_if_selected(paths: &[PathBuf], path: &Path) -> bool {
    for selected_path in paths {
        if path.starts_with(selected_path) {
            return true;
        }
    }
    false
}
