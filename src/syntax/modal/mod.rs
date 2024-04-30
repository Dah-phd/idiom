use crate::render::layout::Rect;
use crate::syntax::Lexer;
use crate::syntax::{DiagnosticInfo, Lang};
mod completion;
mod info;
mod parser;
mod rename;

use crate::{global_state::GlobalState, workspace::CursorPosition};
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
    pub fn map_and_finish(&mut self, key: &KeyEvent, lang: &Lang, gs: &mut GlobalState) -> ModalMessage {
        match key {
            KeyEvent { code: KeyCode::Esc, .. } => ModalMessage::TakenDone,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            _ => match self {
                Self::AutoComplete(modal) => modal.map(key, lang, gs),
                Self::Info(modal) => modal.map(key, gs),
                Self::RenameVar(modal) => modal.map(key, gs),
            },
        }
    }

    pub fn render_at(&mut self, col: u16, row: u16, gs: &mut GlobalState) -> Option<Rect> {
        match self {
            Self::AutoComplete(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.editor_area.modal_relative(row, col, 60, height);
                if area.height != 0 {
                    modal.render_at(&area, &mut gs.writer).ok()?;
                    return Some(area);
                };
            }
            _ => {} // Self::RenameVar(modal) => {
                    //     if let Some(area) = dynamic_cursor_rect_sized_height(modal.len(), col, row + 1, frame.size()) {
                    //         let len = area.height as usize + 2;
                    //         frame.render_widget(Clear, area);
                    //         modal.render_at(frame, area);
                    //         return Some(len);
                    //     }
                    // }
                    // Self::Info(modal) => {
                    //     if let Some(area) = dynamic_cursor_rect_sized_height(modal.len(), col, row + 1, frame.size()) {
                    //         let len = area.height as usize + 2;
                    //         frame.render_widget(Clear, area);
                    //         modal.render_at(frame, area);
                    //         return Some(len);
                    //     }
                    // }
        }
        None
    }

    pub fn auto_complete(completions: Vec<CompletionItem>, line: String, idx: usize) -> Option<Self> {
        let modal = AutoComplete::new(completions, line, idx);
        if modal.len() != 0 {
            return Some(LSPModal::AutoComplete(modal));
        }
        None
    }

    pub fn actions(actions: DiagnosticInfo) -> Self {
        Self::Info(Info::from_info(actions))
    }

    pub fn from_hover(hover: Hover, line_builder: &Lexer) -> Self {
        Self::Info(Info::from_hover(hover, line_builder))
    }

    pub fn hover_map(&mut self, hover: Hover, line_builder: &Lexer) {
        match self {
            Self::Info(modal) => modal.push_hover(hover, line_builder),
            _ => *self = Self::Info(Info::from_hover(hover, line_builder)),
        }
    }

    pub fn from_signature(signature: SignatureHelp, line_builder: &Lexer) -> Self {
        Self::Info(Info::from_signature(signature, line_builder))
    }

    pub fn signature_map(&mut self, signature: SignatureHelp, line_builder: &Lexer) {
        match self {
            Self::Info(modal) => modal.push_signature(signature, line_builder),
            _ => *self = Self::Info(Info::from_signature(signature, line_builder)),
        }
    }

    pub fn renames_at(c: CursorPosition, title: &str) -> Self {
        Self::RenameVar(RenameVariable::new(c, title))
    }
}
