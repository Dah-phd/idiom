use crate::render::{backend::BackendProtocol, layout::Rect};
use crate::syntax::{DiagnosticInfo, Lang};
mod completion;
mod info;
mod rename;

use crate::{global_state::GlobalState, workspace::CursorPosition};
use completion::AutoComplete;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use info::Info;
use lsp_types::{CompletionItem, Hover, SignatureHelp};
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
                let area = gs.screen_rect.modal_relative(row, col, 60, height);
                if area.height != 0 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(&area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
            Self::RenameVar(modal) => {
                let area = gs.screen_rect.modal_relative(row, col, 60, modal.len() as u16);
                if area.height != 0 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(&area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
            Self::Info(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.screen_rect.modal_relative(row, col, 60, height);
                if area.height != 0 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(&area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
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

    pub fn from_hover(hover: Hover) -> Self {
        Self::Info(Info::from_hover(hover))
    }

    pub fn hover_map(&mut self, hover: Hover) {
        match self {
            Self::Info(modal) => modal.push_hover(hover),
            _ => *self = Self::Info(Info::from_hover(hover)),
        }
    }

    pub fn from_signature(signature: SignatureHelp) -> Self {
        Self::Info(Info::from_signature(signature))
    }

    pub fn signature_map(&mut self, signature: SignatureHelp) {
        match self {
            Self::Info(modal) => modal.push_signature(signature),
            _ => *self = Self::Info(Info::from_signature(signature)),
        }
    }

    pub fn renames_at(c: CursorPosition, title: &str) -> Self {
        Self::RenameVar(RenameVariable::new(c, title))
    }
}
