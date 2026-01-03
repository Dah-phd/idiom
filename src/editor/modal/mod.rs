mod completion;
mod info;
mod rename;

use crate::{
    configs::{EditorAction, Theme},
    cursor::{CursorPosition, PositionedWord, WordRange},
    editor::{syntax::DiagnosticInfo, Editor, EditorLine},
    global_state::GlobalState,
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
    Skipped,
    Done,
    Taken,
    TakenDone,
    Action(ModalAction),
}

#[derive(Debug, PartialEq)]
pub enum ModalAction {
    Rename(String, CursorPosition),
    AutoComplete(String),
    Snippet { snippet: String, cursor_offset: Option<(usize, usize)>, relative_select: Option<((usize, usize), usize)> },
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
    pub fn is_rendered(&self) -> bool {
        self.last_render.is_some()
    }

    #[inline]
    pub fn forece_render_if_exists(&mut self, relative_pos: Position, gs: &mut GlobalState) {
        let Some(modal) = self.inner.as_mut() else { return };
        self.last_render = modal.render_at(relative_pos, gs);
    }

    #[inline]
    pub fn render_if_exist(&mut self, relative_pos: Position, gs: &mut GlobalState) {
        let Some(modal) = self.inner.as_mut() else { return };
        if self.last_render.is_none() {
            self.last_render = modal.render_at(relative_pos, gs);
        };
    }

    #[inline]
    pub fn paste_if_exists(&mut self, clip: &str) -> (bool, Option<Rect>) {
        let Some(modal) = self.inner.as_mut() else {
            return (false, None);
        };
        match modal {
            LSPModal::RenameVar(modal) => {
                modal.paste(clip);
                (true, self.last_render.take())
            }
            LSPModal::Info(..) => {
                self.inner.take();
                (false, self.last_render.take())
            }
            LSPModal::AutoComplete(..) => {
                self.inner.take();
                (false, self.last_render.take())
            }
        }
    }

    #[inline]
    pub fn map_if_exists(editor: &mut Editor, action: EditorAction, gs: &mut GlobalState) -> (bool, Option<Rect>) {
        let Some(modal) = editor.modal.inner.as_mut() else {
            return (false, None);
        };
        let message = match action {
            EditorAction::Cancel | EditorAction::Close => ModalMessage::TakenDone,
            _ => match modal {
                LSPModal::AutoComplete(modal) => modal.map(action, &editor.lexer.lang, gs),
                LSPModal::Info(modal) => modal.map(action, gs),
                LSPModal::RenameVar(modal) => modal.map(action, gs),
            },
        };
        match message {
            ModalMessage::Skipped => (false, editor.modal.last_render.take()),
            ModalMessage::Taken => (true, editor.modal.last_render.take()),
            ModalMessage::TakenDone => {
                editor.modal.inner.take();
                (true, editor.modal.last_render.take())
            }
            ModalMessage::Done => {
                editor.modal.inner.take();
                (false, editor.modal.last_render.take())
            }
            ModalMessage::Action(action) => {
                match action {
                    ModalAction::Rename(new_name, c) => {
                        if let Err(error) = editor.lexer.try_lsp_rename(c, new_name.to_owned()) {
                            if !error.is_missing_capability() {
                                gs.error(error);
                            }
                            if let Some(old_name) = PositionedWord::find_at(&editor.content, c) {
                                let content_iter = editor.content.iter().enumerate().rev();
                                let ranges = old_name.iter_word_ranges(content_iter).map(|r| r.as_select()).collect();
                                editor.mass_replace(ranges, new_name);
                            }
                        };
                        editor.modal.inner.take();
                    }
                    ModalAction::AutoComplete(new) => editor.replace_token(new),
                    ModalAction::Snippet { snippet, cursor_offset, relative_select } => match relative_select {
                        Some((cursor_offset, len)) => {
                            editor.insert_snippet_with_select(snippet, cursor_offset, len);
                        }
                        None => editor.insert_snippet(snippet, cursor_offset),
                    },
                }
                editor.modal.inner.take();
                (true, editor.modal.last_render.take())
            }
        }
    }

    #[inline]
    pub fn cleanr_render_cache(&mut self) {
        self.last_render = None;
    }

    #[inline]
    pub fn drop(&mut self) -> Option<Rect> {
        _ = self.inner.take();
        self.last_render.take()
    }

    pub fn mouse_click_if_exists(
        editor: &mut Editor,
        relative_editor_position: Position,
        gs: &mut GlobalState,
    ) -> Option<Rect> {
        let modal = editor.modal.inner.as_mut()?;
        let found_positon = editor.modal.last_render.and_then(|rect| {
            let row = gs.editor_area().row + relative_editor_position.row;
            let column = gs.editor_area().col + relative_editor_position.col;
            rect.relative_position(row, column)
        });
        match found_positon {
            // click outside modal
            None => {
                editor.modal.inner.take();
                editor.modal.last_render.take()
            }
            Some(position) => {
                let finish = match modal {
                    LSPModal::AutoComplete(modal) => {
                        match modal.mouse_click_and_finished(position.row as usize, &editor.lexer.lang, gs) {
                            ModalAction::Rename(..) => {}
                            ModalAction::AutoComplete(new) => editor.replace_token(new),
                            ModalAction::Snippet { snippet, cursor_offset, relative_select } => match relative_select {
                                Some((cursor_offset, len)) => {
                                    editor.insert_snippet_with_select(snippet, cursor_offset, len);
                                }
                                None => editor.insert_snippet(snippet, cursor_offset),
                            },
                        }
                        true
                    }
                    LSPModal::Info(modal) => modal.mouse_click_and_finish(position.row as usize, gs),
                    LSPModal::RenameVar(modal) => {
                        if position.row == 1 {
                            modal.mouse_click(position.col as usize);
                            false
                        } else {
                            true
                        }
                    }
                };
                match finish {
                    // modal finished
                    true => {
                        editor.modal.inner.take();
                        editor.modal.last_render.take()
                    }
                    false => editor.modal.last_render.take(),
                }
            }
        }
    }

    pub fn mouse_moved_if_exists(&mut self, row: u16, column: u16) -> Option<Rect> {
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
    pub fn mouse_moved(&mut self, position: Position) -> bool {
        match self {
            Self::AutoComplete(modal) => modal.mouse_moved(position.row as usize),
            Self::Info(modal) => modal.mouse_moved(position.row as usize),
            Self::RenameVar(..) => false,
        }
    }

    pub fn render_at(&mut self, relative_pos: Position, gs: &mut GlobalState) -> Option<Rect> {
        let Position { row: row_offset, col: col_offset } = relative_pos;
        match self {
            Self::AutoComplete(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.editor_area().modal_relative(row_offset, col_offset, 70, height);
                if area.height != 0 {
                    gs.backend.set_style(gs.ui_theme.accent_style());
                    modal.render(&area, gs);
                    gs.backend.reset_style();
                    return Some(area);
                };
            }
            Self::RenameVar(modal) => {
                let area = gs.editor_area().modal_relative(row_offset, col_offset, 60, modal.len() as u16);
                if area.height == 2 {
                    gs.backend.set_style(gs.ui_theme.accent_style());
                    modal.render(&area, gs);
                    gs.backend.reset_style();
                    return Some(area);
                };
            }
            Self::Info(modal) => {
                let height = std::cmp::min(modal.len() as u16, 7);
                let area = gs.editor_area().modal_relative(row_offset, col_offset, 90, height);
                if area.height != 0 {
                    gs.backend.set_style(gs.ui_theme.accent_style());
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
