use super::{
    utils::{into_message, next_option, prev_option},
    PopupInterface,
};
use crate::{
    global_state::{Clipboard, PopupMessage, WorkspaceEvent},
    widgests::{right_corner_rect_static, TextField},
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Default)]
pub struct GoToLinePopup {
    line_idx: String,
    auto_jump: bool,
}

impl GoToLinePopup {
    pub fn new() -> Box<Self> {
        Box::default()
    }
}

impl PopupInterface for GoToLinePopup {
    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard) -> PopupMessage {
        match key.code {
            KeyCode::Enter => {
                if self.auto_jump {
                    return PopupMessage::Clear;
                }
                if let Ok(idx) = self.line_idx.parse::<usize>() {
                    return WorkspaceEvent::GoToLine(idx.saturating_sub(1)).into();
                }
                PopupMessage::Clear
            }
            KeyCode::Char(ch) => {
                if ch.is_numeric() {
                    self.line_idx.push(ch);
                    if self.auto_jump {
                        return WorkspaceEvent::PopupAccess.into();
                    };
                };
                PopupMessage::None
            }
            KeyCode::Backspace => {
                if self.line_idx.pop().is_some() && self.auto_jump {
                    return WorkspaceEvent::PopupAccess.into();
                };
                PopupMessage::None
            }
            KeyCode::Tab => {
                self.auto_jump = !self.auto_jump;
                PopupMessage::None
            }
            _ => PopupMessage::Clear,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = right_corner_rect_static(50, 3, frame.size());
        let block = if self.auto_jump {
            Block::default().title("Autojump (Tab to switch back)")
        } else {
            Block::default().title("Go to line (Tab to switch mode)")
        }
        .borders(Borders::ALL);
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new(format!(" >> {}|", self.line_idx)).block(block), area);
    }

    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(editor) = workspace.get_active() {
            if let Ok(idx) = self.line_idx.parse::<usize>() {
                editor.go_to(idx.saturating_sub(1));
            }
        }
    }
}

pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<PopupMessage>,
    pub state: usize,
}

impl FindPopup {
    pub fn new() -> Box<Self> {
        Box::new(Self { options: Vec::new(), pattern: TextField::with_editor_access(String::new()), state: 0 })
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
        self.state = self.options.len().saturating_sub(1);
    }
}
