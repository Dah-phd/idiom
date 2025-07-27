pub mod diagnostics;
pub mod langs;
pub mod legend;
mod lsp_calls;
pub mod modal;
// pub mod theme;
pub mod tokens;
use crate::{
    configs::{EditorAction, FileType, Theme},
    global_state::{GlobalState, IdiomEvent},
    lsp::{LSPClient, LSPError, LSPResponseType, LSPResult},
    workspace::{
        actions::{EditMetaData, EditType},
        line::EditorLine,
        CursorPosition, Editor,
    },
};
pub use diagnostics::{set_diganostics, Action, DiagnosticInfo, DiagnosticLine};
use idiom_tui::{layout::Rect, Position};
pub use langs::Lang;
pub use legend::Legend;
use lsp_calls::{
    as_url, char_lsp_pos, completable_dead, context_local, encode_pos_utf32, get_autocomplete_dead, info_position_dead,
    map_lsp, remove_lsp, renames_dead, start_renames_dead, sync_changes_dead, sync_edits_dead, sync_edits_dead_rev,
    sync_tokens_dead, tokens_dead, tokens_partial_dead,
};
use lsp_types::{PublishDiagnosticsParams, Range, TextDocumentContentChangeEvent, Uri};
use modal::{LSPModal, ModalMessage};
use std::path::{Path, PathBuf};
pub use tokens::Token;

pub struct Lexer {
    pub lang: Lang,
    pub legend: Legend,
    pub theme: Theme,
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp: bool,
    pub uri: Uri,
    pub path: PathBuf,
    question_lsp: bool,
    version: i32,
    modal: Option<LSPModal>,
    modal_rect: Option<Rect>,
    requests: Vec<LSPResponseType>,
    client: LSPClient,
    context: fn(&mut Editor, &mut GlobalState),
    completable: fn(&Self, char_idx: usize, line: &EditorLine) -> bool,
    autocomplete: fn(&mut Self, CursorPosition, String, &mut GlobalState),
    tokens: fn(&mut Self) -> LSPResult<LSPResponseType>,
    tokens_partial: fn(&mut Self, Range, usize) -> LSPResult<LSPResponseType>,
    references: fn(&mut Self, CursorPosition, &mut GlobalState),
    definitions: fn(&mut Self, CursorPosition, &mut GlobalState),
    declarations: fn(&mut Self, CursorPosition, &mut GlobalState),
    hover: fn(&mut Self, CursorPosition, &mut GlobalState),
    signatures: fn(&mut Self, CursorPosition, &mut GlobalState),
    start_renames: fn(&mut Self, CursorPosition, &str),
    renames: fn(&mut Self, CursorPosition, String, &mut GlobalState),
    sync_tokens: fn(&mut Self, EditMetaData),
    sync_changes: fn(&mut Self, Vec<TextDocumentContentChangeEvent>) -> LSPResult<()>,
    sync: fn(&mut Self, &EditType, &mut [EditorLine]) -> LSPResult<()>,
    sync_rev: fn(&mut Self, &EditType, &mut [EditorLine]) -> LSPResult<()>,
    meta: Option<EditMetaData>,
    pub encode_position: fn(usize, &str) -> usize,
    pub char_lsp_pos: fn(char) -> usize,
}

impl Lexer {
    pub fn with_context(file_type: FileType, path: &Path, gs: &mut GlobalState) -> Self {
        Self {
            lang: Lang::from(file_type),
            legend: Legend::default(),
            theme: gs.unwrap_or_default(Theme::new(), "theme.json: "),
            modal: None,
            modal_rect: None,
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_dead,
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
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encode_position: encode_pos_utf32,
            char_lsp_pos,
            question_lsp: false,
        }
    }

    pub fn text_lexer(path: &Path, gs: &mut GlobalState) -> Self {
        Self {
            lang: Lang::default(),
            legend: Legend::default(),
            theme: gs.unwrap_or_default(Theme::new(), "theme.json: "),
            modal: None,
            modal_rect: None,
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_dead,
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
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encode_position: encode_pos_utf32,
            char_lsp_pos,
            question_lsp: false,
        }
    }

    pub fn md_lexer(path: &Path, gs: &mut GlobalState) -> Self {
        Self {
            lang: Lang::default(),
            legend: Legend::default(),
            theme: gs.unwrap_or_default(Theme::new(), "theme.json: "),
            modal: None,
            modal_rect: None,
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_dead,
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
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encode_position: encode_pos_utf32,
            char_lsp_pos,
            question_lsp: false,
        }
    }

    #[inline]
    pub fn context(editor: &mut Editor, gs: &mut GlobalState) {
        (editor.lexer.context)(editor, gs);
    }

    #[inline]
    pub fn refresh_lsp(&mut self, gs: &mut GlobalState) {
        self.requests.clear();
        self.client.clear_requests();
        match (self.tokens)(self) {
            Ok(request) => self.requests.push(request),
            Err(error) => gs.error(error),
        }
    }

    /// sync tokens from LSP shorthand for request call
    pub fn sync_tokens(&mut self, meta: EditMetaData) {
        (self.sync_tokens)(self, meta);
    }

    pub fn sync_changes(&mut self, change_events: Vec<TextDocumentContentChangeEvent>) {
        self.question_lsp = (self.sync_changes)(self, change_events).is_err();
    }

    /// sync event
    #[inline(always)]
    pub fn sync(&mut self, action: &EditType, content: &mut [EditorLine]) {
        self.question_lsp = (self.sync)(self, action, content).is_err();
    }

    /// sync reverse event
    #[inline(always)]
    pub fn sync_rev(&mut self, action: &EditType, content: &mut [EditorLine]) {
        self.question_lsp = (self.sync_rev)(self, action, content).is_err();
    }

    // MODAL

    #[inline]
    pub fn modal_is_rendered(&self) -> bool {
        self.modal_rect.is_some()
    }

    #[inline]
    pub fn forece_modal_render_if_exists(&mut self, row: u16, col: u16, gs: &mut GlobalState) {
        self.modal_rect = self.modal.as_mut().and_then(|modal| modal.render_at(col, row, gs));
    }

    #[inline]
    pub fn render_modal_if_exist(&mut self, row: u16, col: u16, gs: &mut GlobalState) {
        if self.modal_rect.is_none() {
            self.modal_rect = self.modal.as_mut().and_then(|modal| modal.render_at(col, row, gs));
        };
    }

    #[inline]
    pub fn map_modal_if_exists(&mut self, action: EditorAction, gs: &mut GlobalState) -> (bool, Option<Rect>) {
        let Some(modal) = &mut self.modal else {
            return (false, None);
        };
        match modal.map_and_finish(action, &self.lang, gs) {
            ModalMessage::Taken => (true, self.modal_rect.take()),
            ModalMessage::TakenDone => {
                self.modal.take();
                (true, self.modal_rect.take())
            }
            ModalMessage::Done => {
                self.modal.take();
                (false, self.modal_rect.take())
            }
            ModalMessage::RenameVar(new_name, c) => {
                self.get_rename(c, new_name, gs);
                self.modal.take();
                (true, self.modal_rect.take())
            }
            ModalMessage::None => (false, self.modal_rect.take()),
        }
    }

    #[inline]
    pub fn clear_modal(&mut self) -> Option<Rect> {
        _ = self.modal.take();
        self.modal_rect.take()
    }

    pub fn mouse_click_modal_if_exists(
        &mut self,
        relative_editor_position: Position,
        gs: &mut GlobalState,
    ) -> Option<Rect> {
        let modal = self.modal.as_mut()?;
        let position = self.modal_rect.and_then(|rect| {
            let row = gs.editor_area.row + relative_editor_position.row;
            let column = gs.editor_area.col + relative_editor_position.col;
            rect.relative_position(row, column)
        })?;
        if modal.mouse_click_and_finished(position, &self.lang, gs) {
            self.modal.take();
        };
        self.modal_rect.take()
    }

    pub fn mouse_moved_modal_if_exists(&mut self, row: u16, column: u16) -> Option<Rect> {
        let modal = self.modal.as_mut()?;
        let position = self.modal_rect.and_then(|rect| rect.relative_position(row, column))?;
        modal.mouse_moved(position).then_some(self.modal_rect.take()?)
    }

    // LSP HANDLES

    pub fn set_lsp_client(&mut self, mut client: LSPClient, content: String, gs: &mut GlobalState) {
        if let Err(error) = client.file_did_open(self.uri.clone(), self.lang.file_type, content) {
            gs.error(error);
            return;
        }
        map_lsp(self, client);
        gs.success("LSP mapped!");
        match (self.tokens)(self) {
            Ok(request) => self.requests.push(request),
            Err(err) => gs.send_error(err, self.lang.file_type),
        };
    }

    pub fn local_lsp(&mut self, file_type: FileType, content: String, gs: &mut GlobalState) {
        let client = LSPClient::local_lsp(file_type);
        map_lsp(self, client);
        match self.client.file_did_open(self.uri.clone(), file_type, content) {
            Ok(_) => {
                gs.success("Starting local LSP - internal system to provide basic language feature");
                match (self.tokens)(self) {
                    Ok(request) => self.requests.push(request),
                    Err(err) => gs.send_error(err, file_type),
                }
            }
            // can be reached only due to internal code issue
            Err(error) => {
                gs.error(error);
                remove_lsp(self);
            }
        };
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
    pub fn should_autocomplete(&self, char_idx: usize, line: &EditorLine) -> bool {
        (self.completable)(self, char_idx, line)
    }

    #[inline]
    pub fn get_autocomplete(&mut self, c: CursorPosition, line: String, gs: &mut GlobalState) {
        (self.autocomplete)(self, c, line, gs)
    }

    #[inline]
    pub fn help(&mut self, c: CursorPosition, content: &[EditorLine], gs: &mut GlobalState) {
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
                self.legend.map_styles(self.lang.file_type, &self.theme, capabilities);
            }
            match (self.tokens)(self) {
                Ok(request) => self.requests.push(request),
                Err(error) => gs.send_error(error, self.lang.file_type),
            };
        };
    }

    pub fn save_and_check_lsp(&mut self, content: String, gs: &mut GlobalState) {
        if self.lsp {
            gs.message("Checking LSP status (on save) ...");
            if self.client.file_did_save(self.uri.clone(), content).is_err() && self.client.is_closed() {
                gs.event.push(IdiomEvent::CheckLSP(self.lang.file_type));
            } else {
                gs.success("LSP running ...");
            }
            match (self.tokens)(self) {
                Ok(request) => self.requests.push(request),
                Err(error) => gs.send_error(error, self.lang.file_type),
            };
        }
    }

    pub fn reopen(&mut self, content: String, file_type: FileType) -> Result<(), LSPError> {
        if !self.lsp {
            return Ok(());
        };
        if let Ok(request) = (self.tokens)(self) {
            self.requests.push(request);
        }
        self.client.file_did_open(self.uri.clone(), file_type, content)
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
