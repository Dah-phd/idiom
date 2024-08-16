pub mod diagnostics;
pub mod langs;
pub mod legend;
mod lsp_calls;
pub mod modal;
pub mod theme;
pub mod token;
use crate::{
    configs::FileType,
    global_state::{GlobalState, WorkspaceEvent},
    lsp::{LSPClient, LSPError, LSPResponseType},
    render::layout::Rect,
    workspace::{
        line::{CodeLine, EditorLine},
        CodeEditor, CursorPosition,
    },
};
use crossterm::event::KeyEvent;
pub use diagnostics::{set_diganostics, Action, DiagnosticInfo, DiagnosticLine};
pub use langs::Lang;
pub use legend::Legend;
use lsp_calls::{
    as_url, char_lsp_pos, context_local, encode_pos_utf32, get_autocomplete_dead, info_position_dead, map,
    renames_dead, start_renames_dead, sync_edits_local, tokens_dead, tokens_partial_dead,
};
use lsp_types::{PublishDiagnosticsParams, Range, Uri};
use modal::{LSPModal, ModalMessage};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use theme::Theme;
pub use token::{Token, TokensType};

pub struct Lexer {
    pub lang: Lang,
    pub legend: Legend,
    pub theme: Theme,
    pub token_producer: TokensType,
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp: bool,
    pub uri: Uri,
    pub path: PathBuf,
    clock: Instant,
    modal: Option<LSPModal>,
    modal_rect: Option<Rect>,
    requests: Vec<LSPResponseType>,
    client: LSPClient,
    context: fn(&mut CodeEditor, &mut GlobalState),
    autocomplete: fn(&mut Lexer, CursorPosition, String, &mut GlobalState),
    tokens: fn(&mut Lexer, &mut GlobalState),
    tokens_partial: fn(&mut Lexer, Range, usize, &mut GlobalState),
    references: fn(&mut Lexer, CursorPosition, &mut GlobalState),
    definitions: fn(&mut Lexer, CursorPosition, &mut GlobalState),
    declarations: fn(&mut Lexer, CursorPosition, &mut GlobalState),
    hover: fn(&mut Lexer, CursorPosition, &mut GlobalState),
    signatures: fn(&mut Lexer, CursorPosition, &mut GlobalState),
    start_renames: fn(&mut Lexer, CursorPosition, &str),
    renames: fn(&mut Lexer, CursorPosition, String, &mut GlobalState),
    sync: fn(&mut CodeEditor, &mut GlobalState),
    pub encode_position: fn(usize, &str) -> usize,
    char_lsp_pos: fn(char) -> usize,
}

impl Lexer {
    pub fn with_context(file_type: FileType, path: &Path, gs: &mut GlobalState) -> Self {
        Self {
            lang: Lang::from(file_type),
            legend: Legend::default(),
            theme: gs.unwrap_or_default(Theme::new(), "theme.json: "),
            token_producer: TokensType::Internal,
            clock: Instant::now(),
            modal: None,
            modal_rect: None,
            uri: as_url(path),
            path: path.into(),
            requests: Vec::new(),
            diagnostics: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            autocomplete: get_autocomplete_dead,
            tokens: tokens_dead,
            tokens_partial: tokens_partial_dead,
            references: info_position_dead,
            definitions: info_position_dead,
            declarations: info_position_dead,
            hover: info_position_dead,
            signatures: info_position_dead,
            start_renames: start_renames_dead,
            renames: renames_dead,
            sync: sync_edits_local,
            encode_position: encode_pos_utf32,
            char_lsp_pos,
        }
    }

    #[inline]
    pub fn context(editor: &mut CodeEditor, gs: &mut GlobalState) {
        (editor.lexer.sync)(editor, gs);
        (editor.lexer.context)(editor, gs);
    }

    #[inline]
    pub fn sync(editor: &mut CodeEditor, gs: &mut GlobalState) {
        (editor.lexer.sync)(editor, gs);
    }

    #[inline]
    pub fn render_modal_if_exist(&mut self, row: u16, col: u16, gs: &mut GlobalState) {
        if self.modal_rect.is_none() {
            self.modal_rect = self.modal.as_mut().and_then(|modal| modal.render_at(col, row, gs));
        };
    }

    pub fn map_modal_if_exists(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> (bool, Option<Rect>) {
        if let Some(modal) = &mut self.modal {
            match modal.map_and_finish(key, &self.lang, gs) {
                ModalMessage::Taken => return (true, self.modal_rect.take()),
                ModalMessage::TakenDone => {
                    self.modal.take();
                    return (true, self.modal_rect.take());
                }
                ModalMessage::Done => {
                    self.modal.take();
                    return (false, self.modal_rect.take());
                }
                ModalMessage::RenameVar(new_name, c) => {
                    self.get_rename(c, new_name, gs);
                    self.modal.take();
                    return (true, self.modal_rect.take());
                }
                ModalMessage::None => {
                    return (false, self.modal_rect.take());
                }
            }
        }
        (false, None)
    }

    pub fn set_lsp_client(&mut self, mut client: LSPClient, content: String, gs: &mut GlobalState) {
        if client.file_did_open(self.uri.clone(), self.lang.file_type, content).is_err() {
            return;
        }
        map(self, client);
        gs.success("LSP mapped!");
        (self.tokens)(self, gs);
    }

    pub fn update_path(&mut self, path: &Path) -> Result<(), LSPError> {
        self.path = path.into();
        let old_uri = std::mem::replace(&mut self.uri, as_url(path));
        if self.lsp {
            return self.client.update_path(old_uri, self.uri.clone());
        }
        Ok(())
    }

    #[inline(always)]
    pub fn char_lsp_pos(&self, ch: char) -> usize {
        (self.char_lsp_pos)(ch)
    }

    #[inline]
    pub fn should_autocomplete(&mut self, char_idx: usize, line: &impl EditorLine) -> bool {
        self.lsp && self.lang.completable(line, char_idx) && !matches!(self.modal, Some(LSPModal::AutoComplete(..)))
    }

    #[inline]
    pub fn get_autocomplete(&mut self, c: CursorPosition, line: String, gs: &mut GlobalState) {
        (self.autocomplete)(self, c, line, gs)
    }

    #[inline]
    pub fn help(&mut self, c: CursorPosition, content: &[CodeLine], gs: &mut GlobalState) {
        if let Some(actions) = content[c.line].diagnostic_info(&self.lang) {
            self.modal.replace(LSPModal::actions(actions));
        }
        (self.signatures)(self, c, gs);
        (self.hover)(self, c, gs);
    }

    #[inline]
    pub fn start_rename(&mut self, c: CursorPosition, title: &str) {
        (self.start_renames)(self, c, title);
    }

    #[inline]
    pub fn get_rename(&mut self, c: CursorPosition, new_name: String, gs: &mut GlobalState) {
        (self.renames)(self, c, new_name, gs);
    }

    #[inline]
    pub fn go_to_declaration(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        (self.declarations)(self, c, gs);
    }

    #[inline]
    pub fn go_to_reference(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        (self.references)(self, c, gs);
    }

    pub fn reload_theme(&mut self, gs: &mut GlobalState) {
        self.theme = match Theme::new() {
            Ok(theme) => theme,
            Err(err) => {
                let mut msg = "theme.json: ".to_owned();
                msg.push_str(&err.to_string());
                gs.error(msg);
                return;
            }
        };
        if self.lsp {
            if let Some(capabilities) = &self.client.capabilities.semantic_tokens_provider {
                self.legend.map_styles(&self.lang.file_type, &self.theme, capabilities);
            }
            (self.tokens)(self, gs);
        };
    }

    pub fn save_and_check_lsp(&mut self, content: String, gs: &mut GlobalState) {
        // self.line_builder.mark_saved();
        if self.lsp {
            gs.message("Checking LSP status (on save) ...");
            if self.client.file_did_save(self.uri.clone(), content).is_err() && self.client.is_closed() {
                gs.workspace.push(WorkspaceEvent::CheckLSP(self.lang.file_type));
            } else {
                gs.success("LSP running ...");
            }
            (self.tokens)(self, gs);
        }
    }

    pub fn close(&mut self) {
        if !self.lsp {
            return;
        }
        let _ = self.client.file_did_close(self.uri.clone());
    }
}

#[cfg(test)]
pub mod tests;
