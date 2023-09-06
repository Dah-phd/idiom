mod file_system;
mod tree_paths;
use crate::configs::GeneralAction;
use file_system::FileSystem;
use std::path::PathBuf;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, ListState},
    Frame,
};

pub struct Tree {
    active: bool,
    pub fs: FileSystem,
    pub on_open_tabs: bool,
    _state: ListState,
}

impl Tree {
    pub fn new(active: bool) -> Self {
        Self { active, fs: FileSystem::default(), on_open_tabs: false, _state: ListState::default() }
    }

    pub fn render_with_remainder(&mut self, frame: &mut Frame<impl Backend>, screen: Rect) -> Rect {
        if !self.active {
            return screen;
        }

        let areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(15), Constraint::Min(2)])
            .split(screen);

        let block = Block::default().borders(Borders::ALL).title("Explorer");
        let file_tree = self.fs.as_widget().block(block);
        frame.render_stateful_widget(file_tree, areas[0], &mut self._state);
        areas[1]
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        self.fs.open()
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn get_first_selected_folder(&mut self) -> String {
        if let Some(selected) = self.fs.get_selected() {
            if selected.is_dir() {
                return selected.path().display().to_string();
            }
            if let Some(parent) = selected.parent() {
                return parent.path().display().to_string();
            }
        }
        "./".to_owned()
    }

    pub fn create_file_or_folder(&mut self, name: String) {
        self.fs.new_file_or_folder(name);
    }

    pub fn create_file_or_folder_base(&mut self, name: String) {
        self.fs.new_file_or_folder_base(name);
    }

    pub fn rename_file(&mut self, new_name: String) {
        self.fs.rename(&new_name);
    }

    fn delete_file(&mut self) -> Option<()> {
        self.fs.delete().ok()
    }

    pub fn map(&mut self, action: &GeneralAction) -> bool {
        match action {
            GeneralAction::Up => {
                self.on_open_tabs = false;
                self.fs.select_up()
            }
            GeneralAction::Down => {
                self.on_open_tabs = false;
                self.fs.select_down();
            }
            GeneralAction::Shrink => self.fs.close(),
            GeneralAction::DeleteFile => {
                self.delete_file();
            }
            _ => return false,
        }
        true
    }
}
