use crate::{
    global_state::GlobalState,
    lsp::{LSPClient, LSPResponse, LSPResponseType},
    popups::popups_tree::refrence_selector,
    syntax::Lexer,
    workspace::{actions::EditMetaData, line::EditorLine, CodeEditor, CursorPosition},
};
use core::str::FromStr;
use lsp_types::{
    Range, SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

use super::{
    modal::LSPModal,
    set_diganostics,
    token::{set_tokens, set_tokens_partial},
    TokensType,
};

/// timeout before remapping all tokens
const FULL_TOKENS: Duration = Duration::from_secs(10);

/// maps LSP state without runtime checks
#[inline]
pub fn map(lexer: &mut Lexer, client: LSPClient) {
    lexer.lsp = true;
    lexer.context = context;
    lexer.autocomplete = get_autocomplete;

    // tokens
    if let Some(tc) = client.capabilities.semantic_tokens_provider.as_ref() {
        lexer.legend.map_styles(&lexer.lang.file_type, &lexer.theme, tc);
        lexer.tokens = tokens;
        if client.capabilities.semantic_tokens_provider.as_ref().map(range_tokens_are_supported).unwrap_or_default() {
            lexer.tokens_partial = tokens_partial;
        } else {
            lexer.tokens_partial = tokens_partial_redirect;
        }
    } else {
        lexer.tokens = tokens_dead;
        lexer.tokens_partial = tokens_partial_redirect;
    }

    // definitions
    if client.capabilities.definition_provider.is_some() {
        lexer.definitions = definitions;
    } else {
        lexer.definitions = info_position_dead;
    }

    // references
    if client.capabilities.references_provider.is_some() {
        lexer.references = references;
    } else {
        lexer.references = info_position_dead;
    }

    // declarations
    if client.capabilities.declaration_provider.is_some() {
        lexer.declarations = declarations;
    } else {
        lexer.declarations = info_position_dead;
    }

    // renames
    if client.capabilities.rename_provider.is_some() {
        lexer.start_renames = start_renames;
        lexer.renames = renames;
    } else {
        lexer.start_renames = start_renames_dead;
    }

    // hover
    if client.capabilities.hover_provider.is_some() {
        lexer.hover = hover;
    } else {
        lexer.hover = info_position_dead;
    }

    // sig help
    if client.capabilities.signature_help_provider.is_some() {
        lexer.signatures = signatures;
    }

    // document syncing
    if let Some(sync) = client.capabilities.text_document_sync.as_ref() {
        match sync {
            TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)
            | TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                ..
            }) => {
                lexer.sync = sync_edits;
            }
            _ => {
                lexer.sync = sync_edits_full;
            }
        }
    } else {
        lexer.sync = sync_edits_local;
    }

    match client.capabilities.position_encoding.as_ref().map(|encode| encode.as_str()) {
        Some("utf-8") => {
            lexer.encode_position = encode_pos_utf8;
            lexer.char_lsp_pos = char_lsp_utf8;
        }
        Some("utf-32") => {
            lexer.encode_position = encode_pos_utf32;
            lexer.char_lsp_pos = char_lsp_pos;
        }
        _ => {
            lexer.encode_position = encode_pos_utf16;
            lexer.char_lsp_pos = char_lsp_utf16;
        }
    }

    lexer.client = client;
}

pub fn context_local(_: &mut CodeEditor, _: &mut GlobalState) {}

pub fn context(editor: &mut CodeEditor, gs: &mut GlobalState) {
    let lexer = &mut editor.lexer;
    let client = &mut lexer.client;
    let content = &mut editor.content;

    // diagnostics
    if let Some(diagnostics) = client.get_diagnostics(&editor.path) {
        set_diganostics(content, diagnostics);
    }
    // responses
    let unresolved_requests = &mut lexer.requests;
    for request in std::mem::take(unresolved_requests) {
        if let Some(response) = client.get(request.id()) {
            match request.parse(response.result) {
                Some(result) => match result {
                    LSPResponse::Completion(completions, line, idx) => {
                        lexer.modal = LSPModal::auto_complete(completions, line, idx);
                    }
                    LSPResponse::Hover(hover) => {
                        if let Some(modal) = lexer.modal.as_mut() {
                            modal.hover_map(hover, &lexer.lang, &lexer.theme);
                        } else {
                            lexer.modal.replace(LSPModal::from_hover(hover, &lexer.lang, &lexer.theme));
                        }
                    }
                    LSPResponse::SignatureHelp(signature) => {
                        if let Some(modal) = lexer.modal.as_mut() {
                            modal.signature_map(signature, &lexer.lang, &lexer.theme);
                        } else {
                            lexer.modal.replace(LSPModal::from_signature(signature, &lexer.lang, &lexer.theme));
                        }
                    }
                    LSPResponse::Renames(workspace_edit) => {
                        gs.workspace.push(workspace_edit.into());
                    }
                    LSPResponse::Tokens(tokens) => {
                        match tokens {
                            SemanticTokensResult::Partial(data) => {
                                set_tokens(data.data, &lexer.legend, &lexer.lang, &lexer.theme, content);
                            }
                            SemanticTokensResult::Tokens(data) => {
                                if !data.data.is_empty() {
                                    set_tokens(data.data, &lexer.legend, &lexer.lang, &lexer.theme, content);
                                    lexer.token_producer = TokensType::LSP;
                                    gs.success("LSP tokens mapped!");
                                } else if let Ok(id) = client.request_full_tokens(lexer.uri.clone()) {
                                    unresolved_requests.push(LSPResponseType::Tokens(id));
                                };
                            }
                        };
                    }
                    LSPResponse::TokensPartial { result, max_lines } => {
                        let tokens = match result {
                            SemanticTokensRangeResult::Partial(data) => data.data,
                            SemanticTokensRangeResult::Tokens(data) => data.data,
                        };
                        set_tokens_partial(tokens, max_lines, &lexer.legend, &lexer.lang, &lexer.theme, content);
                    }
                    LSPResponse::References(locations) => {
                        if let Some(mut locations) = locations {
                            if locations.len() == 1 {
                                gs.tree.push(locations.remove(0).into());
                            } else {
                                gs.popup(refrence_selector(locations));
                            }
                        }
                    }
                    LSPResponse::Declaration(declaration) => {
                        gs.try_tree_event(declaration);
                    }
                    LSPResponse::Definition(definition) => {
                        gs.try_tree_event(definition);
                    }
                },
                None => {
                    if let Some(err) = response.error {
                        gs.error(format!("{request}: {err}"));
                    }
                }
            }
        } else {
            unresolved_requests.push(request);
        }
    }
}

pub fn sync_edits(editor: &mut CodeEditor, gs: &mut GlobalState) {
    let (version, events) = match editor.actions.get_events() {
        Some(data) => data,
        None => return,
    };
    let lexer = &mut editor.lexer;
    let content = &editor.content;
    if lexer.clock.elapsed() > FULL_TOKENS && lexer.modal.is_none() {
        let result = lexer.client.sync(
            lexer.uri.clone(),
            version,
            events
                .drain(..)
                .map(|(_, mut edit)| {
                    edit.lsp_encode(lexer.encode_position, content);
                    edit
                })
                .collect(),
        );
        gs.log_if_lsp_error(result, editor.file_type);
        (lexer.tokens)(lexer, gs);
        lexer.clock = Instant::now();
        return;
    }
    let mut change_events = Vec::new();
    let meta = events
        .drain(..)
        .map(|(meta, mut edit)| {
            edit.lsp_encode(lexer.encode_position, content);
            change_events.push(edit);
            meta
        })
        .reduce(|em1, em2| em1 + em2)
        .expect("Value is checked");
    gs.log_if_lsp_error(lexer.client.sync(lexer.uri.clone(), version, change_events), editor.file_type);
    let max_lines = meta.start_line + meta.to;
    (lexer.tokens_partial)(lexer, meta.range(content), max_lines, gs);
}

pub fn sync_edits_full(editor: &mut CodeEditor, gs: &mut GlobalState) {
    let (version, events) = match editor.actions.get_events() {
        Some(data) => data,
        None => return,
    };
    let lexer = &mut editor.lexer;
    let content = &editor.content;
    let mut text = String::new();
    for editor_line in content.iter() {
        editor_line.push_content_to_buffer(&mut text);
        text.push('\n');
    }
    text.push('\n');
    gs.log_if_lsp_error(lexer.client.full_sync(lexer.uri.clone(), version, text), editor.file_type);
    if lexer.clock.elapsed() > FULL_TOKENS && lexer.modal.is_none() {
        events.clear();
        (lexer.tokens)(lexer, gs);
        lexer.clock = Instant::now();
        return;
    };
    let meta = events.drain(..).map(|(meta, _)| meta).reduce(|em1, em2| em1 + em2).expect("Value is checked");
    let max_lines = meta.start_line + meta.to;
    (lexer.tokens_partial)(lexer, meta.range(content), max_lines, gs)
}

pub fn sync_edits_local(editor: &mut CodeEditor, _: &mut GlobalState) {
    let events = match editor.actions.get_events_versionless() {
        Some(events) => events,
        None => return,
    };
    if let Some(meta) = events.drain(..).map(|(meta, ..)| meta).reduce(|left, right| left + right) {
        for line in editor.content.iter_mut().skip(meta.start_line).take(meta.to) {
            line.rebuild_tokens(&editor.lexer);
        }
    }
}

pub fn get_autocomplete(lexer: &mut Lexer, c: CursorPosition, line: String, gs: &mut GlobalState) {
    match lexer.client.request_completions(lexer.uri.clone(), c).map(|id| LSPResponseType::Completion(id, line, c.char))
    {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn get_autocomplete_dead(_: &mut Lexer, _: CursorPosition, _: String, _: &mut GlobalState) {}

pub fn tokens(lexer: &mut Lexer, gs: &mut GlobalState) {
    match lexer.client.request_full_tokens(lexer.uri.clone()).map(LSPResponseType::Tokens) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn tokens_dead(_: &mut Lexer, _: &mut GlobalState) {}

pub fn tokens_partial(lexer: &mut Lexer, range: Range, max_lines: usize, gs: &mut GlobalState) {
    match lexer
        .client
        .request_partial_tokens(lexer.uri.clone(), range)
        .map(|id| LSPResponseType::TokensPartial { id, max_lines })
    {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn tokens_partial_redirect(lexer: &mut Lexer, _: Range, _: usize, gs: &mut GlobalState) {
    tokens(lexer, gs)
}

pub fn tokens_partial_dead(_: &mut Lexer, _: Range, _: usize, _: &mut GlobalState) {}

pub fn info_position_dead(_: &mut Lexer, _: CursorPosition, _: &mut GlobalState) {}

pub fn references(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_references(lexer.uri.clone(), c).map(LSPResponseType::References) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn definitions(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_definitions(lexer.uri.clone(), c).map(LSPResponseType::Definition) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn declarations(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_declarations(lexer.uri.clone(), c).map(LSPResponseType::Declaration) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn hover(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_hover(lexer.uri.clone(), c).map(LSPResponseType::Hover) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn signatures(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_signitures(lexer.uri.clone(), c).map(LSPResponseType::SignatureHelp) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn start_renames_dead(_: &mut Lexer, _: CursorPosition, _: &str) {}

pub fn start_renames(lexer: &mut Lexer, c: CursorPosition, title: &str) {
    lexer.modal.replace(LSPModal::renames_at(c, title));
}

pub fn renames_dead(_: &mut Lexer, _: CursorPosition, _: String, _: &mut GlobalState) {}

pub fn renames(lexer: &mut Lexer, c: CursorPosition, new_name: String, gs: &mut GlobalState) {
    match lexer.client.request_rename(lexer.uri.clone(), c, new_name).map(LSPResponseType::Renames) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

// UTILS

#[inline]
fn range_tokens_are_supported(provider: &SemanticTokensServerCapabilities) -> bool {
    match provider {
        SemanticTokensServerCapabilities::SemanticTokensOptions(opt) => opt.range.unwrap_or_default(),
        SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(data) => {
            data.semantic_tokens_options.range.unwrap_or_default()
        }
    }
}

#[inline]
pub fn encode_pos_utf8(char_idx: usize, from_str: &str) -> usize {
    from_str.char_indices().take(char_idx).last().map(|(idx, _)| idx).unwrap_or_default()
}

#[inline]
pub fn encode_pos_utf16(char_idx: usize, from_str: &str) -> usize {
    from_str.chars().take(char_idx).fold(0, |sum, ch| sum + ch.len_utf16())
}

#[inline]
pub fn encode_pos_utf32(char_idx: usize, _: &str) -> usize {
    char_idx
}

#[inline]
pub fn char_lsp_pos(_: char) -> usize {
    1
}

#[inline]
pub fn char_lsp_utf8(ch: char) -> usize {
    ch.len_utf8()
}

#[inline]
pub fn char_lsp_utf16(ch: char) -> usize {
    ch.len_utf16()
}

#[inline(always)]
pub fn as_url(path: &Path) -> Uri {
    Uri::from_str(format!("file://{}", path.display()).as_str()).expect("Path should always be parsable!")
}

impl EditMetaData {
    #[inline]
    pub fn range(self, content: &[impl EditorLine]) -> lsp_types::Range {
        let end_line = self.end_line();
        let end_character = content.get(end_line).map(|l| l.char_len()).unwrap_or_default() as u32;
        lsp_types::Range::new(
            lsp_types::Position::new(self.start_line as u32, 0),
            lsp_types::Position::new(end_line as u32, end_character),
        )
    }
}
