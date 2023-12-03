use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    components::{workspace::CursorPosition, Tree, Workspace},
    events::{messages::PopupMessage, WorkspaceEvent},
    utils::right_corner_rect_static,
};

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
    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage {
        match key.code {
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::None;
                }
                WorkspaceEvent::ReplaceNextSelect(self.new_text.to_owned(), self.drain_next()).into()
            }
            KeyCode::Char('a' | 'A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::None;
                }
                WorkspaceEvent::ReplaceAll(self.new_text.to_owned(), self.options.clone()).into()
            }
            KeyCode::Char(ch) => {
                self.push(ch);
                WorkspaceEvent::PopupAccess.into()
            }
            KeyCode::Backspace => {
                self.backspace();
                WorkspaceEvent::PopupAccess.into()
            }
            KeyCode::Tab => {
                self.on_text = !self.on_text;
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Enter => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc | KeyCode::Left => PopupMessage::Done,
            _ => PopupMessage::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = right_corner_rect_static(100, 3, frame.size());
        let block = Block::default().title("Replace").borders(Borders::ALL);
        let mut spans = vec![
            Span::raw(count_as_string(&self.options)),
            Span::raw(" >> "),
            Span::raw(self.pattern.to_owned()),
            Span::raw(" <> "),
            Span::raw(self.new_text.to_owned()),
        ];
        if self.on_text {
            spans.push(Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)));
        } else {
            spans.insert(3, Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)));
        }
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    }

    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(editor) = workspace.get_active() {
            self.options.clear();
            editor.find(&self.pattern, &mut self.options);
        }
        self.state = self.options.len().checked_sub(1).unwrap_or_default();
    }

    fn update_tree(&mut self, _: &mut Tree) {}
}
