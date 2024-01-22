mod completion;
mod info;
mod parser;
mod rename;

use crate::{global_state::GlobalState, widgests::dynamic_cursor_rect_sized_height, workspace::CursorPosition};
use completion::AutoComplete;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use info::Info;
use lsp_types::{CompletionItem, Hover, SignatureHelp};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{widgets::Clear, Frame};
use rename::RenameVariable;

pub enum LSPModal {
    AutoComplete(AutoComplete),
    RenameVar(RenameVariable),
    Info(Info),
}

#[derive(Default, Debug)]
pub enum ModalMessage {
    Taken,
    #[default]
    None,
    Done,
    TakenDone,
    RenameVar(String, CursorPosition),
}

impl<T> From<&[T]> for ModalMessage {
    fn from(value: &[T]) -> Self {
        if value.is_empty() {
            ModalMessage::Done
        } else {
            ModalMessage::default()
        }
    }
}

impl LSPModal {
    pub fn map_and_finish(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        match key {
            KeyEvent { code: KeyCode::Esc, .. } => ModalMessage::TakenDone,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            _ => match self {
                Self::AutoComplete(modal) => modal.map(key, gs),
                Self::Info(modal) => modal.map(key, gs),
                Self::RenameVar(modal) => modal.map(key, gs),
            },
        }
    }

    pub fn render_at(&mut self, frame: &mut Frame, x: u16, y: u16) {
        match self {
            Self::AutoComplete(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(modal.len(), x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
            Self::RenameVar(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(1, x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
            Self::Info(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(modal.len(), x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
        }
    }

    pub fn auto_complete(completions: Vec<CompletionItem>, line: String, idx: usize) -> Option<Self> {
        let modal = AutoComplete::new(completions, line, idx);
        if modal.len() != 0 {
            return Some(LSPModal::AutoComplete(modal));
        }
        None
    }

    pub fn actions(actions: Vec<String>) -> Self {
        Self::Info(Info::from_imports(actions))
    }

    pub fn hover_map(&mut self, hover: Hover) {
        match self {
            Self::Info(modal) => modal.push_hover(hover),
            _ => *self = hover.into(),
        }
    }

    pub fn signature_map(&mut self, signature: SignatureHelp) {
        match self {
            Self::Info(modal) => modal.push_signature(signature),
            _ => *self = signature.into(),
        }
    }

    pub fn renames_at(c: CursorPosition, title: &str) -> Self {
        Self::RenameVar(RenameVariable::new(c, title))
    }
}

impl From<Hover> for LSPModal {
    fn from(hover: Hover) -> Self {
        Self::Info(hover.into())
    }
}

impl From<SignatureHelp> for LSPModal {
    fn from(signature: SignatureHelp) -> Self {
        Self::Info(signature.into())
    }
}
