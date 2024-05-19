use super::{
    utils::{into_message, next_option, prev_option},
    PopupInterface,
};
use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage, WorkspaceEvent},
    render::{
        backend::{BackendProtocol, Style},
        count_as_string, TextField,
    },
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct GoToLinePopup {
    line_idx: String,
}

impl GoToLinePopup {
    pub fn new() -> Box<Self> {
        Box::default()
    }
}

impl PopupInterface for GoToLinePopup {
    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard) -> PopupMessage {
        match key.code {
            KeyCode::Char(ch) => {
                if ch.is_numeric() {
                    self.line_idx.push(ch);
                    return WorkspaceEvent::PopupAccess.into();
                };
                PopupMessage::None
            }
            KeyCode::Backspace => {
                if self.line_idx.pop().is_some() {
                    return WorkspaceEvent::PopupAccess.into();
                };
                PopupMessage::None
            }
            _ => PopupMessage::Clear,
        }
    }

    fn render(&mut self, gs: &mut GlobalState) {
        if let Some(line) = gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            gs.writer.set_style(gs.theme.accent_style);
            let mut builder = line.unsafe_builder(&mut gs.writer);
            builder.push(" Go to >> ");
            builder.push(&self.line_idx);
            builder.push_styled("|", Style::slowblink());
            drop(builder);
            gs.writer.reset_style();
        };
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

    fn render(&mut self, gs: &mut GlobalState) {
        if let Some(line) = gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            gs.writer.set_style(gs.theme.accent_style);
            let mut builder = line.unsafe_builder(&mut gs.writer);
            builder.push(" Found(");
            builder.push(&count_as_string(self.options.len()));
            builder.push(") >> ");
            self.pattern.insert_formatted_text(builder);
            gs.writer.reset_style();
        }
    }

    fn update_workspace(&mut self, workspace: &mut Workspace) {
        if let Some(editor) = workspace.get_active() {
            self.options.clear();
            editor.find(self.pattern.text.as_str(), &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
    }
}
