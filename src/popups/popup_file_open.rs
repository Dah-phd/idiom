use super::PopupInterface;
use crate::{
    global_state::{Clipboard, IdiomEvent, PopupMessage},
    render::{backend::Backend, layout::Rect, state::State, TextField},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;
use std::{
    fs::DirEntry,
    path::{PathBuf, MAIN_SEPARATOR},
};

pub struct OpenFileSelector {
    pattern: TextField<bool>,
    updated: bool,
    state: State,
    rect: Option<Rect>,
    paths: Vec<String>,
}

impl OpenFileSelector {
    pub fn boxed() -> Box<dyn PopupInterface> {
        let path = dirs::home_dir().unwrap_or(std::env::current_dir().unwrap_or(PathBuf::from("./")));
        let mut text = path.display().to_string();
        if path.is_dir() && !text.ends_with(MAIN_SEPARATOR) {
            text.push(MAIN_SEPARATOR)
        }
        let pattern = TextField::new(text, Some(true));
        let mut new = Self { updated: true, pattern, state: State::new(), paths: vec![], rect: None };
        new.solve_comletions();
        Box::new(new)
    }

    fn solve_comletions(&mut self) {
        self.paths.clear();
        self.state.reset();
        let path = PathBuf::from(&self.pattern.text);
        match path.is_dir() {
            true => {
                if let Ok(entries) = path.read_dir() {
                    self.paths.extend(entries.flatten().map(|de| de.path().display().to_string()));
                }
            }
            false => {
                if let Some(entries) = path.parent().and_then(|parent| parent.read_dir().ok()) {
                    self.paths.extend(entries.flatten().filter_map(|de| checked_string(de, &self.pattern.text)));
                }
            }
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
    fn render(&mut self, screen: Rect, backend: &mut Backend) {
        let mut rect = screen.top(15).vcenter(100);
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, backend);
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, backend),
            None => return,
        }
        match self.paths.is_empty() {
            true => {
                self.state.render_list(["No child paths found!"].into_iter(), rect, backend);
            }
            false => {
                self.state.render_list(self.paths.iter().map(String::as_str), rect, backend);
            }
        };
    }

    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        self.mark_as_updated();
        PopupMessage::None
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        if self.state.selected != 0 {
            if let KeyEvent { code: KeyCode::Enter | KeyCode::Tab, .. } = key {
                let mut text = self.paths.remove(self.state.selected);
                if PathBuf::from(&text).is_dir() && !text.ends_with(MAIN_SEPARATOR) {
                    text.push(MAIN_SEPARATOR);
                }
                self.pattern.text_set(text);
                self.solve_comletions();
                return PopupMessage::None;
            }
        }
        if let Some(updated) = self.pattern.map(key, clipboard) {
            if updated {
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

    fn mouse_map(&mut self, event: crossterm::event::MouseEvent) -> PopupMessage {
        let (row, column) = match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => (row, column),
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.paths.len());
                self.mark_as_updated();
                return PopupMessage::None;
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.paths.len());
                self.mark_as_updated();
                return PopupMessage::None;
            }
            _ => return PopupMessage::None,
        };
        let relative_row = match self.rect.as_ref().and_then(|rect| rect.relative_position(row, column)) {
            Some(pos) => pos.line,
            None => return PopupMessage::None,
        };
        if relative_row < 1 {
            return PopupMessage::None;
        }
        let path_index = self.state.at_line + (relative_row - 1);
        if self.paths.len() <= path_index {
            return PopupMessage::None;
        }
        let mut text = self.paths.remove(path_index);
        let path = PathBuf::from(&text);
        if path.is_file() {
            return IdiomEvent::OpenAtLine(path, 0).into();
        }
        if path.is_dir() && !text.ends_with(MAIN_SEPARATOR) {
            text.push(MAIN_SEPARATOR);
        }
        self.pattern.text_set(text);
        self.solve_comletions();
        self.mark_as_updated();
        PopupMessage::None
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        if self.pattern.paste_passthrough(clip) {
            self.mark_as_updated();
            self.solve_comletions();
        }
        PopupMessage::None
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

fn checked_string(de: DirEntry, pattern: &str) -> Option<String> {
    let new_path = de.path().display().to_string();
    match new_path.starts_with(pattern) {
        true => Some(new_path),
        false => None,
    }
}
