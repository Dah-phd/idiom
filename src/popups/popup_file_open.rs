use super::{Components, InplacePopup, Status};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{GlobalState, IdiomEvent},
    render::{layout::Rect, state::State, TextField},
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use std::{
    fs::DirEntry,
    path::{PathBuf, MAIN_SEPARATOR},
};

pub struct OpenFileSelector {
    pattern: TextField<bool>,
    state: State,
    paths: Vec<String>,
}

impl OpenFileSelector {
    pub fn run(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let path = dirs::home_dir().unwrap_or(std::env::current_dir().unwrap_or(PathBuf::from("./")));
        let mut text = path.display().to_string();
        if path.is_dir() && !text.ends_with(MAIN_SEPARATOR) {
            text.push(MAIN_SEPARATOR)
        }
        let pattern = TextField::new(text, Some(true));
        let mut new = Self { pattern, state: State::new(), paths: vec![] };
        new.solve_comletions();
        new.run(gs, ws, tree, term);
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

    fn get_rect(gs: &GlobalState) -> Rect {
        gs.screen_rect.top(15).vcenter(100).with_borders()
    }

    fn get_path_idx(&self, row: u16, column: u16, gs: &GlobalState) -> Option<usize> {
        let CursorPosition { line, .. } = Self::get_rect(gs).relative_position(row, column)?;
        let path_index = self.state.at_line + line.checked_sub(1)?;
        if self.paths.len() <= path_index {
            return None;
        }
        Some(path_index)
    }
}

impl InplacePopup for OpenFileSelector {
    type R = ();

    fn force_render(&mut self, gs: &mut crate::global_state::GlobalState) {
        let mut rect = Self::get_rect(gs);
        let backend = gs.backend();
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

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, .. } = components;

        if self.state.selected != 0 {
            if let KeyEvent { code: KeyCode::Enter | KeyCode::Tab, .. } = key {
                let mut text = self.paths.remove(self.state.selected);
                if PathBuf::from(&text).is_dir() && !text.ends_with(MAIN_SEPARATOR) {
                    text.push(MAIN_SEPARATOR);
                }
                self.pattern.text_set(text);
                self.solve_comletions();
            }
        }
        if let Some(updated) = self.pattern.map(&key, &mut gs.clipboard) {
            if updated {
                self.solve_comletions();
            }
            self.force_render(gs);
            return Status::Pending;
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
                    gs.event.push(IdiomEvent::OpenAtLine(PathBuf::from(self.pattern.text.as_str()), 0));
                    return Status::Dropped;
                }
                self.resolve_completion();
            }
            _ => {}
        }
        self.force_render(gs);
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, .. } = components;

        let (row, column) = match event {
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } => {
                if let Some(path_idx) = self.get_path_idx(row, column, gs) {
                    self.state.select(path_idx, self.paths.len());
                    self.force_render(gs);
                }
                return Status::Pending;
            }
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => (row, column),
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.paths.len());
                self.force_render(gs);
                return Status::Pending;
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.paths.len());
                self.force_render(gs);
                return Status::Pending;
            }
            _ => return Status::Pending,
        };
        let Some(path_index) = self.get_path_idx(row, column, gs) else {
            return Status::Pending;
        };
        let mut text = self.paths.remove(path_index);
        let path = PathBuf::from(&text);
        if path.is_file() {
            gs.event.push(IdiomEvent::OpenAtLine(path, 0));
            return Status::Dropped;
        }
        if path.is_dir() && !text.ends_with(MAIN_SEPARATOR) {
            text.push(MAIN_SEPARATOR);
        }
        self.pattern.text_set(text);
        self.solve_comletions();
        self.force_render(gs);
        Status::Pending
    }

    fn render(&mut self, _: &mut GlobalState) {}

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        if !self.pattern.paste_passthrough(clip) {
            return false;
        }
        self.solve_comletions();
        true
    }

    fn resize_success(&mut self, _: &mut GlobalState) -> bool {
        true
    }
}

fn checked_string(de: DirEntry, pattern: &str) -> Option<String> {
    let new_path = de.path().display().to_string();
    match new_path.starts_with(pattern) {
        true => Some(new_path),
        false => None,
    }
}
