use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    global_state::{messages::PopupMessage, WorkspaceEvent},
    tree::Tree,
    utils::right_corner_rect_static,
    workspace::{CursorPosition, Workspace},
};

use super::{
    utils::{count_as_string, into_message, next_option, prev_option},
    PopupInterface,
};

#[derive(Debug, Default)]
pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: String,
    pub state: usize,
}

impl FindPopup {
    pub fn new() -> Box<Self> {
        Box::default()
    }
}

impl PopupInterface for FindPopup {
    fn key_map(&mut self, key: &KeyEvent) -> PopupMessage {
        match key.code {
            KeyCode::Enter | KeyCode::Down => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                WorkspaceEvent::FindToReplace(self.pattern.to_owned(), self.options.clone()).into()
            }
            KeyCode::Char(ch) => {
                self.pattern.push(ch);
                WorkspaceEvent::PopupAccess.into()
            }
            KeyCode::Backspace => {
                self.pattern.pop();
                WorkspaceEvent::PopupAccess.into()
            }
            KeyCode::Esc | KeyCode::Left => PopupMessage::Done,
            KeyCode::Tab => WorkspaceEvent::FindSelector(self.pattern.to_owned()).into(),
            _ => PopupMessage::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = right_corner_rect_static(50, 3, frame.size());
        let block = Block::default().title("Find").borders(Borders::ALL);
        frame.render_widget(Clear, area);
        let paragrapth = Paragraph::new(Line::from(vec![
            Span::raw(count_as_string(&self.options)),
            Span::raw(" >> "),
            Span::raw(self.pattern.to_owned()),
            Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]));
        frame.render_widget(paragrapth.block(block), area);
    }

    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(editor) = workspace.get_active() {
            self.options.clear();
            editor.find(self.pattern.as_str(), &mut self.options);
        }
        self.state = self.options.len().checked_sub(1).unwrap_or_default();
    }

    fn update_tree(&mut self, _: &mut Tree) {}
}
