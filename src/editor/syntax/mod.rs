pub mod diagnostics;
mod encoding;
pub mod langs;
pub mod legend;
mod lsp_calls;
pub mod tokens;
use crate::{
    actions::{Action, EditMetaData},
    configs::{FileType, Theme},
    cursor::{Cursor, CursorPosition},
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
    as_url, completable_disable, context_local, formatting_dead, get_autocomplete_dead, info_position_dead, map_lsp,
    remove_lsp, sync_changes_dead, sync_edits_dead, sync_edits_rev_dead, sync_tokens_dead, tokens_dead,
    tokens_partial_dead,
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
    version: i32,
    encoding: Encoding,
    client: LSPClient,
    requests: Vec<i64>,

    /// for status check on LSP
    question_lsp: bool,
    completion_cache: Option<CursorPosition>,
    meta: Option<EditMetaData>,

    /// sync editor to lsp
    context: fn(&mut Editor, &mut GlobalState),

    /// LSP request / notification callbacks

    /// check and get if autocomplete is needed
    /// if completable -> autocomplete
    completable: fn(&Self, char_idx: usize, line: &EditorLine) -> bool,
    autocomplete: fn(&mut Self, CursorPosition, &mut GlobalState),

    tokens: fn(&mut Self, &mut GlobalState),
    tokens_partial: fn(&mut Self, Range, usize, &mut GlobalState),

    references: fn(&mut Self, CursorPosition, &mut GlobalState),
    definitions: fn(&mut Self, CursorPosition, &mut GlobalState),
    declarations: fn(&mut Self, CursorPosition, &mut GlobalState),
    hover: fn(&mut Self, CursorPosition, &mut GlobalState),
    signatures: fn(&mut Self, CursorPosition, &mut GlobalState),

    formatting: fn(&mut Self, usize, bool, &mut GlobalState),

    /// SYNC
    /// get partial tokens based on EditMetaData
    sync_tokens: fn(&mut Self, EditMetaData),
    /// sync change events to lsp
    sync_changes: fn(&mut Self, Vec<TextDocumentContentChangeEvent>) -> LSPResult<()>,
    /// sync changes + sync_tokens
    sync: fn(&mut Self, &Action, &[EditorLine]) -> LSPResult<()>,
    /// sync_changes + sync_tokens on reverted action
    sync_rev: fn(&mut Self, &Action, &[EditorLine]) -> LSPResult<()>,
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
            completion_cache: None,
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
            sync_rev: sync_edits_rev_dead,
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
            completion_cache: None,
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
            sync_rev: sync_edits_rev_dead,
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
            completion_cache: None,
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
            sync_rev: sync_edits_rev_dead,
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
        self.client.clear_responses();
        self.completion_cache = None;
        (self.tokens)(self, gs);
    }

    /// sync tokens from LSP shorthand for request call
    #[inline(always)]
    pub fn sync_tokens(&mut self, meta: EditMetaData) {
        (self.sync_tokens)(self, meta);
    }

    #[inline(always)]
    pub fn sync_changes(&mut self, change_events: Vec<TextDocumentContentChangeEvent>) {
        self.question_lsp = (self.sync_changes)(self, change_events).is_err();
    }

    #[inline(always)]
    pub fn sync_changes_from_action(&mut self, action: &Action, content: &[EditorLine]) {
        let changes = action.text_changes(self.encoding.encode_position, self.encoding.char_len, content);
        self.sync_changes(changes);
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
        (self.tokens)(self, gs);
    }

    pub fn local_lsp(&mut self, file_type: FileType, content: String, gs: &mut GlobalState) {
        let client = LSPClient::local_lsp(file_type);
        map_lsp(self, client, &gs.theme);
        match self.client.file_did_open(self.uri.clone(), file_type, content) {
            Ok(_) => {
                gs.success("Starting local LSP - internal system to provide basic language feature");
                (self.tokens)(self, gs);
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
    pub fn is_completable(&mut self, cursor: &Cursor, line: &EditorLine, ch: char) -> bool {
        match self.completion_cache.as_mut() {
            Some(pos) => {
                if pos.line == cursor.line && pos.char + 1 == cursor.char && ch.is_alphabetic() {
                    pos.char = cursor.char;
                    return false;
                }
                self.completion_cache = None;
                (self.completable)(self, cursor.char, line)
            }
            None => (self.completable)(self, cursor.char, line),
        }
    }

    #[inline]
    pub fn get_autocomplete(&mut self, c: CursorPosition, gs: &mut GlobalState) {
        self.completion_cache = Some(c);
        (self.autocomplete)(self, c, gs)
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
            (self.tokens)(self, gs);
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
            (self.tokens)(self, gs);
        }
    }

    pub fn reopen(&mut self, content: String, file_type: FileType, gs: &mut GlobalState) -> Result<(), LSPError> {
        if !self.lsp {
            return Ok(());
        };
        (self.tokens)(self, gs);
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
            sync_rev: std::mem::replace(&mut lexer.sync_rev, lsp_calls::sync_edits_rev_dead),
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
