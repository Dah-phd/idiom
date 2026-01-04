pub mod diagnostics;
mod encoding;
pub mod langs;
pub mod legend;
mod lsp_calls;
pub mod tokens;
use crate::{
    actions::{Action, EditMetaData},
    configs::{FileType, Theme},
    cursor::CursorPosition,
    editor::Editor,
    editor_line::EditorLine,
    global_state::{GlobalState, IdiomEvent},
    lsp::{LSPClient, LSPError, LSPResult},
};
pub use diagnostics::{set_diganostics, DiagnosticInfo, DiagnosticLine, Fix};
pub use encoding::Encoding;
pub use langs::Lang;
pub use legend::Legend;
use lsp_calls::{
    as_url, completable, completable_disable, context_local, formatting_dead, get_autocomplete_dead,
    info_position_dead, map_lsp, remove_lsp, sync_changes_dead, sync_edits_dead, sync_edits_dead_rev, sync_tokens_dead,
    tokens_dead, tokens_partial_dead,
};
use lsp_types::{PublishDiagnosticsParams, Range, TextDocumentContentChangeEvent, Uri};
use std::path::{Path, PathBuf};
pub use tokens::Token;

pub struct Lexer {
    pub lang: Lang,
    pub legend: Legend,
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp: bool,
    pub uri: Uri,
    pub path: PathBuf,
    question_lsp: bool,
    version: i32,
    requests: Vec<i64>,
    client: LSPClient,
    context: fn(&mut Editor, &mut GlobalState),
    completable: fn(&Self, char_idx: usize, line: &EditorLine) -> bool,
    autocomplete: fn(&mut Self, CursorPosition, String, &mut GlobalState),
    tokens: fn(&mut Self) -> LSPResult<i64>,
    tokens_partial: fn(&mut Self, Range, usize) -> LSPResult<i64>,
    references: fn(&mut Self, CursorPosition, &mut GlobalState),
    definitions: fn(&mut Self, CursorPosition, &mut GlobalState),
    declarations: fn(&mut Self, CursorPosition, &mut GlobalState),
    hover: fn(&mut Self, CursorPosition, &mut GlobalState),
    signatures: fn(&mut Self, CursorPosition, &mut GlobalState),
    formatting: fn(&mut Self, usize, bool, &mut GlobalState),
    sync_tokens: fn(&mut Self, EditMetaData),
    sync_changes: fn(&mut Self, Vec<TextDocumentContentChangeEvent>) -> LSPResult<()>,
    sync: fn(&mut Self, &Action, &[EditorLine]) -> LSPResult<()>,
    sync_rev: fn(&mut Self, &Action, &[EditorLine]) -> LSPResult<()>,
    meta: Option<EditMetaData>,
    encoding: Encoding,
}

impl Lexer {
    pub fn with_context(file_type: FileType, path: &Path) -> Self {
        Self {
            lang: Lang::from(file_type),
            legend: Legend::default(),
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_disable,
            autocomplete: get_autocomplete_dead,
            tokens: tokens_dead,
            tokens_partial: tokens_partial_dead,
            references: info_position_dead,
            definitions: info_position_dead,
            declarations: info_position_dead,
            hover: info_position_dead,
            signatures: info_position_dead,
            formatting: formatting_dead,
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encoding: Encoding::utf32(),
            question_lsp: false,
        }
    }

    pub fn text_lexer(path: &Path) -> Self {
        Self {
            lang: Lang::default(),
            legend: Legend::default(),
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_disable,
            autocomplete: get_autocomplete_dead,
            tokens: tokens_dead,
            tokens_partial: tokens_partial_dead,
            references: info_position_dead,
            definitions: info_position_dead,
            declarations: info_position_dead,
            hover: info_position_dead,
            signatures: info_position_dead,
            formatting: formatting_dead,
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encoding: Encoding::utf32(),
            question_lsp: false,
        }
    }

    pub fn md_lexer(path: &Path) -> Self {
        Self {
            lang: Lang::default(),
            legend: Legend::default(),
            uri: as_url(path),
            path: path.into(),
            version: 0,
            requests: Vec::new(),
            diagnostics: None,
            meta: None,
            lsp: false,
            client: LSPClient::placeholder(),
            context: context_local,
            completable: completable_disable,
            autocomplete: get_autocomplete_dead,
            tokens: tokens_dead,
            tokens_partial: tokens_partial_dead,
            references: info_position_dead,
            definitions: info_position_dead,
            declarations: info_position_dead,
            hover: info_position_dead,
            signatures: info_position_dead,
            formatting: formatting_dead,
            sync_tokens: sync_tokens_dead,
            sync_changes: sync_changes_dead,
            sync: sync_edits_dead,
            sync_rev: sync_edits_dead_rev,
            encoding: Encoding::utf32(),
            question_lsp: false,
        }
    }

    #[inline]
    pub fn context(editor: &mut Editor, gs: &mut GlobalState) {
        (editor.lexer.context)(editor, gs);
    }

    #[inline]
    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }

    #[inline]
    pub fn refresh_lsp(&mut self, gs: &mut GlobalState) {
        self.requests.clear();
        self.client.clear_requests();
        if self.client.capabilities.completion_provider.is_some() {
            self.completable = completable;
        }
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
    pub fn sync(&mut self, action: &Action, content: &[EditorLine]) {
        self.question_lsp = (self.sync)(self, action, content).is_err();
    }

    /// sync reverse event
    #[inline(always)]
    pub fn sync_rev(&mut self, action: &Action, content: &mut [EditorLine]) {
        self.question_lsp = (self.sync_rev)(self, action, content).is_err();
    }

    // LSP HANDLES

    pub fn set_lsp_client(&mut self, mut client: LSPClient, content: String, gs: &mut GlobalState) {
        if let Err(error) = client.file_did_open(self.uri.clone(), self.lang.file_type, content) {
            gs.error(error);
            return;
        }
        map_lsp(self, client, &gs.theme);
        gs.success("LSP mapped!");
        match (self.tokens)(self) {
            Ok(request) => self.requests.push(request),
            Err(err) => gs.send_error(err, self.lang.file_type),
        };
    }

    pub fn local_lsp(&mut self, file_type: FileType, content: String, gs: &mut GlobalState) {
        let client = LSPClient::local_lsp(file_type);
        map_lsp(self, client, &gs.theme);
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
        (self.encoding.char_len)(ch)
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
    pub fn help(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        (self.signatures)(self, c, gs);
        (self.hover)(self, c, gs);
    }

    #[inline]
    pub fn try_lsp_rename(&mut self, c: CursorPosition, new_name: String) -> LSPResult<()> {
        if self.client.capabilities.rename_provider.is_none() {
            return Err(LSPError::missing_capability("renames"));
        }
        let request = self.client.request_rename(self.uri.clone(), c, new_name)?;
        self.requests.push(request);
        Ok(())
    }

    #[inline]
    pub fn go_to_declaration(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        (self.declarations)(self, c, gs);
    }

    #[inline]
    pub fn go_to_reference(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        (self.references)(self, c, gs);
    }

    #[inline]
    pub fn formatting(&mut self, indent: usize, save: bool, gs: &mut GlobalState) {
        (self.formatting)(self, indent, save, gs);
    }

    pub fn reload_theme(&mut self, gs: &mut GlobalState) {
        if self.lsp {
            if let Some(capabilities) = &self.client.capabilities.semantic_tokens_provider {
                self.legend.map_styles(self.lang.file_type, &gs.theme, capabilities);
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

pub struct SyncCallbacks {
    sync: fn(&mut Lexer, &Action, &[EditorLine]) -> LSPResult<()>,
    sync_rev: fn(&mut Lexer, &Action, &[EditorLine]) -> LSPResult<()>,
    sync_changes: fn(&mut Lexer, Vec<TextDocumentContentChangeEvent>) -> LSPResult<()>,
    sync_tokens: fn(&mut Lexer, EditMetaData),
}

impl SyncCallbacks {
    pub fn take(lexer: &mut Lexer) -> Self {
        Self {
            sync: std::mem::replace(&mut lexer.sync, lsp_calls::sync_edits_dead),
            sync_rev: std::mem::replace(&mut lexer.sync_rev, lsp_calls::sync_edits_dead_rev),
            sync_changes: std::mem::replace(&mut lexer.sync_changes, lsp_calls::sync_changes_dead),
            sync_tokens: std::mem::replace(&mut lexer.sync_tokens, lsp_calls::sync_tokens_dead),
        }
    }

    pub fn set_in(self, lexer: &mut Lexer) {
        lexer.sync = self.sync;
        lexer.sync_rev = self.sync_rev;
        lexer.sync_changes = self.sync_changes;
        lexer.sync_tokens = self.sync_tokens;
    }
}

#[cfg(test)]
pub mod tests;
