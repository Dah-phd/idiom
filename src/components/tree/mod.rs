mod file_system;
mod tree_paths;
use crate::configs::GeneralAction;
use file_system::FileSystem;
use std::path::PathBuf;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, List},
    Frame,
};

pub struct Tree {
    active: bool,
    pub fs: FileSystem,
    pub on_open_tabs: bool,
    pub input: Option<String>,
}

impl Tree {
    pub fn new(active: bool) -> Self {
        Self { active, fs: FileSystem::default(), on_open_tabs: false, input: Option::default() }
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
        let file_tree = List::new(self.fs.as_widgets(&self.input, self.active)).block(block);
        frame.render_widget(file_tree, areas[0]);
        areas[1]
    }

    pub fn expand_dir_or_get_path(&mut self) -> Option<PathBuf> {
        self.fs.open()
    }

    pub fn map(&mut self, action: &GeneralAction) -> bool {
        if self.input.is_none() {
            self.map_tree(action)
        } else {
            self.map_input(action)
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    fn create_new(&mut self) {
        if let Some(name) = self.input.take() {
            self.fs.new_file(name);
        }
    }

    fn delete_file(&mut self) -> Option<()> {
        let path = self.fs.get_selected()?.path();
        if path.is_file() { std::fs::remove_file(path) } else { std::fs::remove_dir_all(path) }.ok()
    }

    fn map_tree(&mut self, action: &GeneralAction) -> bool {
        match action {
            GeneralAction::Up => {
                self.on_open_tabs = false;
                self.fs.select_prev()
            }
            GeneralAction::Down => {
                self.on_open_tabs = false;
                self.fs.select_next();
            }
            GeneralAction::Shrink => self.fs.close(),
            GeneralAction::NewFile => {
                self.input = Some(String::new());
            }
            GeneralAction::DeleteFile => {
                self.delete_file();
            }
            _ => return false,
        }
        true
    }

    fn map_input(&mut self, action: &GeneralAction) -> bool {
        if let Some(input) = &mut self.input {
            match action {
                GeneralAction::Char(ch) => input.push(*ch),
                GeneralAction::BackspaceTreeInput => {
                    input.pop();
                }
                GeneralAction::FileTreeModeOrCancelInput => self.input = None,
                GeneralAction::FinishOrSelect => self.create_new(),
                _ => return false,
            }
            return true;
        }
        false
    }
}
