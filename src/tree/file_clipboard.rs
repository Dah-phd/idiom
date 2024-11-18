use std::{
    collections::HashSet,
    path::{Path, PathBuf},
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
    marked_paths: HashSet<PathBuf>,
    action_type: ActionType,
}

impl FileClipboard {
    pub fn get_mark(&self, path: &Path) -> Option<&str> {
        match self.action_type {
            ActionType::None => None,
            ActionType::Copy => match self.marked_paths.contains(path) {
                true => Some(" c "),
                false => None,
            },
            ActionType::Cut => match self.marked_paths.contains(path) {
                true => Some(" x "),
                false => None,
            },
        }
    }

    pub fn cut(&mut self, path: PathBuf) {
        match self.action_type {
            ActionType::Cut => {
                self.marked_paths.insert(path);
            }
            ActionType::Copy => {
                self.marked_paths.clear();
                self.action_type = ActionType::Cut;
                self.marked_paths.insert(path);
            }
            ActionType::None => {
                self.action_type = ActionType::Cut;
                self.marked_paths.insert(path);
            }
        }
    }

    pub fn force_cut(&mut self, path: PathBuf) {
        self.marked_paths.clear();
        self.marked_paths.insert(path);
        self.action_type = ActionType::Cut;
    }

    pub fn copy(&mut self, path: PathBuf) {
        match self.action_type {
            ActionType::Copy => {
                self.marked_paths.insert(path);
            }
            ActionType::Cut => {
                self.marked_paths.clear();
                self.action_type = ActionType::Copy;
                self.marked_paths.insert(path);
            }
            ActionType::None => {
                self.action_type = ActionType::Copy;
                self.marked_paths.insert(path);
            }
        }
    }

    pub fn force_copy(&mut self, path: PathBuf) {
        self.marked_paths.clear();
        self.marked_paths.insert(path);
        self.action_type = ActionType::Copy;
    }

    pub fn paste<P: AsRef<Path>>(&mut self, selected_path: P) {
        match std::mem::take(&mut self.action_type) {
            ActionType::None => return,
            ActionType::Copy => todo!("Copy func"),
            ActionType::Cut => todo!("Cut func"),
        }
    }
}
