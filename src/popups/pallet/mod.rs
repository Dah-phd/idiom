mod formatting;
use super::{popup_file_open::OpenFileSelector, Command, CommandResult, PopupInterface};
use crate::{
    configs::{EDITOR_CFG_FILE, KEY_MAP, THEME_FILE, THEME_UI},
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{backend::Backend, layout::Rect, state::State, TextField},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub struct Pallet {
    commands: Vec<(i64, Command)>,
    access_cb: Option<fn(&mut Workspace, &mut Tree)>,
    pattern: TextField<bool>,
    updated: bool,
    rect: Option<Rect>,
    state: State,
}

impl PopupInterface for Pallet {
    fn render(&mut self, screen: Rect, backend: &mut Backend) {
        let mut rect = screen.top(15).vcenter(100);
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, backend);
        match rect.next_line() {
            Some(line) => self.pattern.widget(line, backend),
            None => return,
        }
        let options = self.commands.iter().map(|cmd| cmd.1.label);
        self.state.render_list(options, rect, backend);
    }

    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        self.mark_as_updated();
        PopupMessage::None
    }

    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, matcher: &SkimMatcherV2) -> PopupMessage {
        if self.commands.is_empty() {
            return PopupMessage::Clear;
        }

        if let Some(updated) = self.pattern.map(key, clipboard) {
            if updated {
                for (score, cmd) in self.commands.iter_mut() {
                    *score = match matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                        Some(new_score) => new_score,
                        None => i64::MAX,
                    };
                }
                self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
            }
            return PopupMessage::None;
        }
        match key.code {
            KeyCode::Enter => match self.commands.remove(self.state.selected).1.execute() {
                CommandResult::Simple(event) => PopupMessage::ClearEvent(event),
                CommandResult::Complex(cb) => {
                    self.access_cb.replace(cb);
                    PopupMessage::Event(IdiomEvent::PopupAccessOnce)
                }
            },
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.commands.len());
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.commands.len());
                PopupMessage::None
            }
            _ => PopupMessage::None,
        }
    }

    fn paste_passthrough(&mut self, clip: String, matcher: &SkimMatcherV2) -> PopupMessage {
        if self.pattern.paste_passthrough(clip) {
            self.mark_as_updated();
            for (score, cmd) in self.commands.iter_mut() {
                *score = match matcher.fuzzy_match(cmd.label, &self.pattern.text) {
                    Some(new_score) => new_score,
                    None => i64::MAX,
                };
            }
            self.commands.sort_by(|(score, _), (rhscore, _)| score.cmp(rhscore));
        }
        PopupMessage::None
    }

    fn mouse_map(&mut self, event: crossterm::event::MouseEvent) -> PopupMessage {
        let (row, column) = match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } => (row, column),
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.commands.len());
                self.mark_as_updated();
                return PopupMessage::None;
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.commands.len());
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
        let command_index = self.state.at_line + (relative_row - 1);
        if self.commands.len() <= command_index {
            return PopupMessage::None;
        }
        match self.commands.remove(command_index).1.execute() {
            CommandResult::Simple(event) => PopupMessage::ClearEvent(event),
            CommandResult::Complex(cb) => {
                self.access_cb.replace(cb);
                PopupMessage::Event(IdiomEvent::PopupAccessOnce)
            }
        }
    }

    fn mark_as_updated(&mut self) {
        self.updated = true
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }

    fn component_access(&mut self, _gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree) {
        if let Some(cb) = self.access_cb.take() {
            cb(ws, tree);
        }
    }
}

impl Pallet {
    pub fn new(_screen: Rect) -> Box<Self> {
        let commands = [
            Some(Command::pass_event("Open file", IdiomEvent::NewPopup(OpenFileSelector::boxed))),
            Some(Command::pass_event("GitUI", IdiomEvent::EmbededApp(String::from("gitui")))),
            Some(Command::access_edit("UPPERCASE", formatting::uppercase)),
            Some(Command::access_edit("LOWERCASE", formatting::lowercase)),
            Command::cfg_open("Open editor configs", EDITOR_CFG_FILE),
            Command::cfg_open("Open keymap config", KEY_MAP),
            Command::cfg_open("Open theme config", THEME_FILE),
            Command::cfg_open("Open UI theme config", THEME_UI),
            Some(Command::pass_event("Open editor error log", IdiomEvent::OpenLSPErrors)),
        ]
        .into_iter()
        .flatten()
        .map(|cmd| (0, cmd))
        .collect();

        Box::new(Pallet {
            commands,
            access_cb: None,
            pattern: TextField::new(String::new(), Some(true)),
            updated: true,
            rect: None,
            state: State::new(),
        })
    }
}

#[cfg(test)]
mod tests;
