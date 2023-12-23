use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::{
    global_state::{Clipboard, PopupMessage, WorkspaceEvent},
    widgests::{right_corner_rect_static, TextField},
    workspace::{CursorPosition, Workspace},
};

use super::{
    utils::{into_message, next_option, prev_option},
    PopupInterface,
};

pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField,
    pub state: usize,
}

impl FindPopup {
    pub fn new() -> Box<Self> {
        Box::new(Self { options: Vec::new(), pattern: TextField::with_editor_access(), state: 0 })
    }
}

impl PopupInterface for FindPopup {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard) -> PopupMessage {
        if matches!(key.code, KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL)) {
            return WorkspaceEvent::FindToReplace(self.pattern.text.to_owned(), self.options.clone()).into();
        }
        if let Some(event) = self.pattern.map(key, clipboard) {
            return event;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Down => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc | KeyCode::Left => PopupMessage::Clear,
            KeyCode::Tab => WorkspaceEvent::FindSelector(self.pattern.text.to_owned()).into(),
            _ => PopupMessage::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = right_corner_rect_static(50, 3, frame.size());
        let block = Block::default().title("Find").borders(Borders::ALL);
        frame.render_widget(Clear, area);
        frame.render_widget(self.pattern.widget_with_count(self.options.len()).block(block), area);
    }

    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(editor) = workspace.get_active() {
            self.options.clear();
            editor.find(self.pattern.text.as_str(), &mut self.options);
        }
        self.state = self.options.len().checked_sub(1).unwrap_or_default();
    }
}
