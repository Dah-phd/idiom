mod completion;
mod info;
mod rename;

use crate::{
    configs::{EditorAction, Theme},
    global_state::GlobalState,
    syntax::{DiagnosticInfo, Lang},
    workspace::CursorPosition,
};
use completion::AutoComplete;
use fuzzy_matcher::skim::SkimMatcherV2;
use idiom_tui::{layout::Rect, Backend, Position};
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

    pub fn mouse_moved(&mut self, position: Position) -> bool {
        match self {
            Self::AutoComplete(modal) => modal.mouse_moved(position.row as usize),
            Self::Info(modal) => modal.mouse_moved(position.row as usize),
            Self::RenameVar(..) => false,
        }
    }

    pub fn mouse_click_and_finished(&mut self, position: Position, lang: &Lang, gs: &mut GlobalState) -> bool {
        match self {
            Self::AutoComplete(modal) => modal.mouse_click_and_finished(position.row as usize, lang, gs),
            Self::Info(modal) => modal.mouse_click_and_finish(position.row as usize, gs),
            Self::RenameVar(modal) => {
                if position.row == 1 {
                    modal.mouse_click(position.col as usize);
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn render_at(&mut self, col: u16, row: u16, gs: &mut GlobalState) -> Option<Rect> {
        match self {
            Self::AutoComplete(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.editor_area().modal_relative(row, col, 70, height);
                if area.height != 0 {
                    gs.backend.set_style(gs.theme.accent_style());
                    modal.render(&area, gs);
                    gs.backend.reset_style();
                    return Some(area);
                };
            }
            Self::RenameVar(modal) => {
                let area = gs.editor_area().modal_relative(row, col, 60, modal.len() as u16);
                if area.height == 2 {
                    gs.backend.set_style(gs.theme.accent_style());
                    modal.render(&area, gs);
                    gs.backend.reset_style();
                    return Some(area);
                };
            }
            Self::Info(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.editor_area().modal_relative(row, col, 90, height);
                if area.height != 0 {
                    gs.backend.set_style(gs.theme.accent_style());
                    modal.render(area, gs);
                    gs.backend.reset_style();
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
