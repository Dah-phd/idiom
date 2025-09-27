use crate::workspace::editor::WordRange;
use crate::workspace::EditorLine;
mod completion;
mod info;
mod rename;

use crate::{
    configs::{EditorAction, Theme},
    global_state::GlobalState,
    syntax::{DiagnosticInfo, Lang, Lexer},
    workspace::CursorPosition,
};
use completion::AutoComplete;
use fuzzy_matcher::skim::SkimMatcherV2;
use idiom_tui::{layout::Rect, Backend, Position};
use info::Info;
use lsp_types::{CompletionItem, Hover, SignatureHelp};
use rename::RenameVariable;

#[derive(Default, Debug)]
pub enum ModalMessage {
    #[default]
    None,
    Done,
    Taken,
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

pub enum LSPModal {
    AutoComplete(AutoComplete),
    RenameVar(RenameVariable),
    Info(Info),
}

#[derive(Default)]
pub struct EditorModal {
    last_render: Option<Rect>,
    inner: Option<LSPModal>,
}

impl EditorModal {
    #[inline]
    pub fn modal_is_rendered(&self) -> bool {
        self.last_render.is_some()
    }

    #[inline]
    pub fn forece_modal_render_if_exists(&mut self, row: u16, col: u16, gs: &mut GlobalState) {
        let Some(modal) = self.inner.as_mut() else { return };
        self.last_render = modal.render_at(col, row, gs);
    }

    #[inline]
    pub fn render_modal_if_exist(&mut self, row: u16, col: u16, gs: &mut GlobalState) {
        let Some(modal) = self.inner.as_mut() else { return };
        if self.last_render.is_none() {
            self.last_render = modal.render_at(col, row, gs);
        };
    }

    #[inline]
    pub fn map_modal_if_exists(
        &mut self,
        action: EditorAction,
        lexer: &mut Lexer,
        gs: &mut GlobalState,
    ) -> (bool, Option<Rect>) {
        let Some(modal) = self.inner.as_mut() else {
            return (false, None);
        };
        match modal.map_and_finish(action, &lexer.lang, gs) {
            ModalMessage::None => (false, self.last_render.take()),
            ModalMessage::Taken => (true, self.last_render.take()),
            ModalMessage::TakenDone => {
                self.inner.take();
                (true, self.last_render.take())
            }
            ModalMessage::Done => {
                self.inner.take();
                (false, self.last_render.take())
            }
            ModalMessage::RenameVar(new_name, c) => {
                lexer.get_rename(c, new_name, gs);
                self.inner.take();
                (true, self.last_render.take())
            }
        }
    }

    #[inline]
    pub fn cleanr_render_cache(&mut self) {
        self.last_render = None;
    }

    #[inline]
    pub fn clear_modal(&mut self) -> Option<Rect> {
        _ = self.inner.take();
        self.last_render.take()
    }

    pub fn mouse_click_modal_if_exists(
        &mut self,
        relative_editor_position: Position,
        lexer: &Lexer,
        gs: &mut GlobalState,
    ) -> Option<Rect> {
        let modal = self.inner.as_mut()?;
        let found_positon = self.last_render.and_then(|rect| {
            let row = gs.editor_area().row + relative_editor_position.row;
            let column = gs.editor_area().col + relative_editor_position.col;
            rect.relative_position(row, column)
        });
        match found_positon {
            // click outside modal
            None => {
                self.inner.take();
                self.last_render.take()
            }
            Some(position) => match modal.mouse_click_and_finished(position, &lexer.lang, gs) {
                // modal finished
                true => {
                    self.inner.take();
                    self.last_render.take()
                }
                false => self.last_render.take(),
            },
        }
    }

    pub fn mouse_moved_modal_if_exists(&mut self, row: u16, column: u16) -> Option<Rect> {
        let modal = self.inner.as_mut()?;
        let position = self.last_render.and_then(|rect| rect.relative_position(row, column))?;
        modal.mouse_moved(position).then_some(self.last_render.take()?)
    }

    pub fn is_autocomplete(&self) -> bool {
        matches!(self.inner, Some(LSPModal::AutoComplete(..)))
    }

    pub fn replace_with_action(&mut self, actions: DiagnosticInfo) {
        self.inner.replace(LSPModal::actions(actions));
    }

    pub fn map_signatures(&mut self, signature: SignatureHelp, theme: &Theme) {
        if let Some(modal) = self.inner.as_mut() {
            modal.signature_map(signature, theme);
            self.cleanr_render_cache();
        } else {
            self.inner = Some(LSPModal::from_signature(signature, theme));
        }
    }
    pub fn map_hover(&mut self, hover: Hover, theme: &Theme) {
        if let Some(modal) = self.inner.as_mut() {
            modal.hover_map(hover, theme);
            self.cleanr_render_cache();
        } else {
            self.inner = Some(LSPModal::from_hover(hover, theme));
        }
    }

    pub fn auto_complete(
        &mut self,
        completions: Vec<CompletionItem>,
        line: String,
        c: CursorPosition,
        matcher: &SkimMatcherV2,
    ) {
        self.inner = LSPModal::auto_complete(completions, line, c, matcher);
    }

    pub fn start_renames(&mut self, content: &[EditorLine], position: CursorPosition) {
        if let Some(title) = WordRange::find_text_at(content, position) {
            self.inner.replace(LSPModal::renames_at(position, title));
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
