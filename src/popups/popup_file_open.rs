use super::PopupInterface;
use crate::{
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{state::State, TextField},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent};
use std::path::{PathBuf, MAIN_SEPARATOR};

pub struct OpenFileSelector {
    pattern: TextField<bool>,
    updated: bool,
    state: State,
    paths: Vec<String>,
}

impl OpenFileSelector {
    pub fn boxed() -> Box<dyn PopupInterface> {
        let folder =
            dirs::home_dir().unwrap_or(std::env::current_dir().unwrap_or(PathBuf::from("./"))).display().to_string();
        let mut new =
            Self { updated: true, pattern: TextField::new(folder, Some(true)), state: State::new(), paths: vec![] };
        new.solve_comletions();
        Box::new(new)
    }

    fn solve_comletions(&mut self) {
        self.paths.clear();
        self.state.select(0, 1);
        let path = PathBuf::from(&self.pattern.text);
        if !path.is_dir() {
            if let Some(entries) = path.parent().and_then(|parent| parent.read_dir().ok()) {
                self.paths.extend(entries.flatten().filter_map(|p| {
                    let new_path = p.path().display().to_string();
                    match new_path.starts_with(&self.pattern.text) {
                        true => Some(new_path),
                        false => None,
                    }
                }));
            }
            return;
        }
        if let Ok(entries) = path.read_dir() {
            self.paths.extend(entries.flatten().map(|p| p.path().display().to_string()));
        }
    }

    fn resolve_completion(&mut self) {
        let match_idx = self.paths.iter().position(|txt| txt.starts_with(&self.pattern.text));
        if let Some(idx) = match_idx {
            let mut text = self.paths.remove(idx);
            if PathBuf::from(&text).is_dir() {
                text.push(MAIN_SEPARATOR);
            }
            self.pattern.text_set(text);
            self.solve_comletions();
        }
    }
}

impl PopupInterface for OpenFileSelector {
    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if self.state.selected != 0 {
            if let KeyEvent { code: KeyCode::Enter | KeyCode::Tab, .. } = key {
                let mut text = self.paths.remove(self.state.selected);
                if PathBuf::from(&text).is_dir() {
                    text.push(MAIN_SEPARATOR);
                }
                self.pattern.text_set(text);
                self.solve_comletions();
                return PopupMessage::None;
            }
        }
        if let Some(updated) = self.pattern.map(key, clipboard) {
            self.updated = updated;
            if self.updated {
                self.solve_comletions();
            }
            return PopupMessage::None;
        }
        match key {
            KeyEvent { code: KeyCode::Up, .. } => {
                self.state.prev(self.paths.len());
            }
            KeyEvent { code: KeyCode::Down, .. } => {
                self.state.next(self.paths.len());
            }
            KeyEvent { code: KeyCode::Tab, .. } => {
                self.resolve_completion();
            }
            KeyEvent { code: KeyCode::Enter, .. } => {
                let path = PathBuf::from(&self.pattern.text);
                if path.is_file() {
                    return IdiomEvent::OpenAtLine(PathBuf::from(self.pattern.text.as_str()), 0).into();
                }
                self.resolve_completion();
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn component_access(&mut self, _ws: &mut Workspace, _tree: &mut Tree) {}

    fn render(&mut self, gs: &mut GlobalState) {
        let mut rect = gs.screen_rect.top(15).vcenter(100);
        rect.bordered();
        rect.draw_borders(None, None, gs.backend());
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, gs.backend()),
            None => return,
        }
        match self.paths.is_empty() {
            true => {
                self.state.render_list(["No child paths found!"].into_iter(), rect, gs.backend());
            }
            false => {
                self.state.render_list(self.paths.iter().map(String::as_str), rect, gs.backend());
            }
        };
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }
}
