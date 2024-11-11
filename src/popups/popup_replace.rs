use crate::{
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::backend::{BackendProtocol, Style},
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;

use super::{
    utils::{count_as_string, into_message, next_option, prev_option},
    PopupInterface,
};

#[derive(Default)]
pub struct ReplacePopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: String,
    pub new_text: String,
    pub on_text: bool,
    pub state: usize,
}

impl ReplacePopup {
    pub fn new() -> Box<Self> {
        Box::default()
    }

    pub fn from_search(pattern: String, options: Vec<(CursorPosition, CursorPosition)>) -> Box<Self> {
        Box::new(Self { on_text: true, pattern, options, ..Default::default() })
    }

    fn drain_next(&mut self) -> (CursorPosition, CursorPosition) {
        let position = self.options.remove(self.state);
        if self.state >= self.options.len() {
            self.state = 0;
        }
        position
    }

    fn get_state(&self) -> Option<(CursorPosition, CursorPosition)> {
        self.options.get(self.state).cloned()
    }

    fn push(&mut self, ch: char) {
        if self.on_text {
            self.new_text.push(ch);
        } else {
            self.pattern.push(ch);
        };
    }

    fn backspace(&mut self) {
        if self.on_text {
            self.new_text.pop();
        } else {
            self.pattern.pop();
        };
    }
}

impl PopupInterface for ReplacePopup {
    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        match key.code {
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::None;
                }
                IdiomEvent::ReplaceNextSelect {
                    new_text: self.new_text.to_owned(),
                    select: self.drain_next(),
                    next_select: self.get_state(),
                }
                .into()
            }
            KeyCode::Char('a' | 'A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::None;
                }
                IdiomEvent::ReplaceAll(self.new_text.to_owned(), self.options.clone()).into()
            }
            KeyCode::Char(ch) => {
                self.push(ch);
                IdiomEvent::PopupAccess.into()
            }
            KeyCode::Backspace => {
                self.backspace();
                IdiomEvent::PopupAccess.into()
            }
            KeyCode::Tab => {
                self.on_text = !self.on_text;
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Enter => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc | KeyCode::Left => PopupMessage::Clear,
            _ => PopupMessage::None,
        }
    }

    fn fast_render(&mut self, gs: &mut GlobalState) {
        let area = gs.editor_area.right_top_corner(2, 50);
        if area.height < 2 {
            return;
        };
        gs.writer.set_style(gs.theme.accent_style);
        let mut lines = area.into_iter();
        if let Some(line) = lines.next() {
            let mut find_builder = line.unsafe_builder(&mut gs.writer);
            find_builder.push(count_as_string(&self.options).as_str());
            find_builder.push(" > ");
            find_builder.push(&self.pattern);
            if !self.on_text {
                find_builder.push_styled("|", Style::slowblink());
            };
        };
        if let Some(line) = lines.next() {
            let mut repl_builder = line.unsafe_builder(&mut gs.writer);
            repl_builder.push("Rep > ");
            repl_builder.push(&self.new_text);
            if self.on_text {
                repl_builder.push_styled("|", Style::slowblink());
            }
        }
        gs.writer.reset_style();
    }

    fn render(&mut self, gs: &mut GlobalState) {
        self.fast_render(gs);
    }

    fn component_access(&mut self, ws: &mut Workspace, _tree: &mut Tree) {
        if let Some(editor) = ws.get_active() {
            self.options.clear();
            editor.find(&self.pattern, &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
    }

    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn mark_as_updated(&mut self) {}
}
