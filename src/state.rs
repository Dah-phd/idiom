use std::path::PathBuf;

use tui::widgets::ListState;

use crate::messages;

#[derive(Clone, Default)]
pub struct Tree {
    pub expanded: Vec<PathBuf>,
    pub state: ListState,
    pub tree: Vec<PathBuf>,
}

pub struct File {
    location: PathBuf,
    content: Vec<String>,
    err: Option<std::io::Error>,
}

impl File {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        todo!()
    }

    pub fn compare(&self) -> Option<Vec<String>> {
        todo!()
    }

    fn load(&mut self) {
        let content = std::fs::read_to_string(&self.location);
    }
}

pub struct EdiotorState {
    pub editors: Vec<File>,
    pub state: ListState,
}

pub struct State {
    pub mode: messages::Mode,
    pub select: Option<(String, u16, u16)>,
    pub ready_to_exit: bool,
    pub file_tree: Option<Tree>,
    pub buffer: String,
    pub opened_files: EdiotorState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mode: messages::Mode::Select,
            select: None,
            ready_to_exit: false,
            file_tree: Some(Tree::default()),
            buffer: String::new(),
            opened_files: EdiotorState {
                editors: vec![],
                state: ListState::default(),
            },
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn switch_tree(&mut self) {
        self.file_tree = if self.file_tree.is_none() {
            Some(Tree::default())
        } else {
            None
        }
    }

    pub fn save_all(&mut self) {}
    pub fn save_current(&mut self) {}
}
