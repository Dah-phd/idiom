use std::{collections::HashSet, path::PathBuf};

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

    pub fn paste(&mut self, path: PathBuf) {}
}
