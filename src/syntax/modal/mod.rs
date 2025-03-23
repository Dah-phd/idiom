mod completion;
mod info;
mod rename;

use crate::{
    configs::{EditorAction, Theme},
    global_state::GlobalState,
    render::{backend::BackendProtocol, layout::Rect},
    syntax::{DiagnosticInfo, Lang},
    workspace::CursorPosition,
};
use completion::AutoComplete;
use fuzzy_matcher::skim::SkimMatcherV2;
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
    pub fn map_and_finish(&mut self, action: EditorAction, lang: &Lang, gs: &mut GlobalState) -> ModalMessage {
        match action {
            EditorAction::Cancel | EditorAction::Close => ModalMessage::TakenDone,
            _ => match self {
                Self::AutoComplete(modal) => modal.map(action, lang, gs),
                Self::Info(modal) => modal.map(action, gs),
                Self::RenameVar(modal) => modal.map(action, gs),
            },
        }
    }

    pub fn render_at(&mut self, col: u16, row: u16, gs: &mut GlobalState) -> Option<Rect> {
        match self {
            Self::AutoComplete(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.screen_rect.modal_relative(row, col, 50, height);
                if area.height != 0 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(&area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
            Self::RenameVar(modal) => {
                let area = gs.screen_rect.modal_relative(row, col, 60, modal.len() as u16);
                if area.height == 2 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(&area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
            Self::Info(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.screen_rect.modal_relative(row, col, 80, height);
                if area.height != 0 {
                    gs.writer.set_style(gs.theme.accent_style);
                    modal.render(area, gs);
                    gs.writer.reset_style();
                    return Some(area);
                };
            }
        }
        None
    }

    pub fn auto_complete(
        completions: Vec<CompletionItem>,
        line: String,
        c: CursorPosition,
        matcher: &SkimMatcherV2,
    ) -> Option<Self> {
        let modal = AutoComplete::new(completions, line, c, matcher);
        if modal.len() != 0 {
            return Some(LSPModal::AutoComplete(modal));
        }
        None
    }

    pub fn actions(actions: DiagnosticInfo) -> Self {
        Self::Info(Info::from_info(actions))
    }

    pub fn from_hover(hover: Hover, theme: &Theme) -> Self {
        Self::Info(Info::from_hover(hover, theme))
    }

    pub fn hover_map(&mut self, hover: Hover, theme: &Theme) {
        match self {
            Self::Info(modal) => modal.push_hover(hover, theme),
            _ => *self = Self::Info(Info::from_hover(hover, theme)),
        }
    }

    pub fn from_signature(signature: SignatureHelp, theme: &Theme) -> Self {
        Self::Info(Info::from_signature(signature, theme))
    }

    pub fn signature_map(&mut self, signature: SignatureHelp, theme: &Theme) {
        match self {
            Self::Info(modal) => modal.push_signature(signature, theme),
            _ => *self = Self::Info(Info::from_signature(signature, theme)),
        }
    }

    pub fn renames_at(c: CursorPosition, title: &str) -> Self {
        Self::RenameVar(RenameVariable::new(c, title))
    }
}
