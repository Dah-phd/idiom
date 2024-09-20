use crate::{
    global_state::GlobalState,
    lsp::{LSPClient, LSPResponse, LSPResponseType, LSPResult},
    popups::popups_tree::refrence_selector,
    syntax::Lexer,
    workspace::{actions::EditType, line::EditorLine, CursorPosition, Editor},
};
use core::str::FromStr;
use lsp_types::{
    Range, SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use std::path::Path;

use super::{
    modal::LSPModal,
    set_diganostics,
    tokens::{set_tokens, set_tokens_partial},
};

/// maps LSP state without runtime checks
#[inline]
pub fn map(lexer: &mut Lexer, client: LSPClient) {
    lexer.lsp = true;

    lexer.context = context;

    if let Some(provider) = client.capabilities.completion_provider.as_ref() {
        lexer.autocomplete = get_autocomplete;
        lexer.completable = completable;
        if let Some(chars) = provider.trigger_characters.as_ref() {
            if !chars.is_empty() {
                lexer.lang.compl_trigger_chars.clear();
                for string in chars {
                    lexer.lang.compl_trigger_chars.push_str(string);
                }
            }
        }
    }

    // tokens
    if let Some(tc) = client.capabilities.semantic_tokens_provider.as_ref() {
        lexer.legend.map_styles(lexer.lang.file_type, &lexer.theme, tc);
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
                lexer.sync_rev = sync_edits_rev;
            }
            _ => {
                lexer.sync = sync_edits_full;
                lexer.sync = sync_edits_full_rev;
            }
        }
    } else {
        lexer.sync = sync_edits_dead;
        lexer.sync_rev = sync_edits_dead_rev;
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

pub fn remove_lsp(lexer: &mut Lexer) {
    lexer.lsp = false;
    lexer.client = LSPClient::placeholder();
    lexer.context = context_local;
    lexer.completable = completable_dead;
    lexer.autocomplete = get_autocomplete_dead;
    lexer.tokens = tokens_dead;
    lexer.tokens_partial = tokens_partial_dead;
    lexer.references = info_position_dead;
    lexer.definitions = info_position_dead;
    lexer.declarations = info_position_dead;
    lexer.hover = info_position_dead;
    lexer.signatures = info_position_dead;
    lexer.start_renames = start_renames_dead;
    lexer.renames = renames_dead;
    lexer.sync = sync_edits_dead;
    lexer.sync_rev = sync_edits_dead_rev;
    lexer.encode_position = encode_pos_utf32;
    lexer.char_lsp_pos = char_lsp_pos;
}

pub fn context_local(_: &mut Editor, _: &mut GlobalState) {}

pub fn context(editor: &mut Editor, gs: &mut GlobalState) {
    let lexer = &mut editor.lexer;
    let client = &mut lexer.client;
    let content = &mut editor.content;

    // diagnostics
    if let Some(diagnostics) = client.get_diagnostics(&editor.path) {
        set_diganostics(content, diagnostics);
    }
    // responses
    if let Some(mut responses) = client.get_responses() {
        let unresolved_requests = &mut lexer.requests;
        for request in std::mem::take(unresolved_requests) {
            if let Some(response) = responses.remove(request.id()) {
                match request.parse(response.result) {
                    Some(result) => match result {
                        LSPResponse::Completion(completions, line, c) => {
                            if editor.cursor.line == c.line {
                                lexer.modal = LSPModal::auto_complete(completions, line, c);
                            }
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
                            gs.event.push(workspace_edit.into());
                        }
                        LSPResponse::Tokens(tokens) => {
                            match tokens {
                                SemanticTokensResult::Partial(data) => {
                                    set_tokens(data.data, &lexer.legend, &lexer.theme, content);
                                }
                                SemanticTokensResult::Tokens(data) => {
                                    set_tokens(data.data, &lexer.legend, &lexer.theme, content);
                                    gs.success("LSP tokens mapped! Refresh UI to remove artifacts (default F5)");
                                }
                            };
                        }
                        LSPResponse::TokensPartial { result, max_lines } => {
                            let tokens = match result {
                                SemanticTokensRangeResult::Partial(data) => data.data,
                                SemanticTokensRangeResult::Tokens(data) => data.data,
                            };
                            set_tokens_partial(tokens, max_lines, &lexer.legend, &lexer.theme, content);
                        }
                        LSPResponse::References(locations) => {
                            if let Some(mut locations) = locations {
                                if locations.len() == 1 {
                                    gs.event.push(locations.remove(0).into());
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
                if matches!(request, LSPResponseType::Tokens(..)) {
                    lexer.meta = None;
                    crate::lsp::init_local_tokens(editor.file_type, content, &lexer.theme);
                }
                unresolved_requests.push(request);
            }
        }
    }

    if let Some(meta) = lexer.meta.take() {
        let max_lines = (meta.start_line + meta.to) - 1;
        if max_lines >= content.len() {
            return;
        }
        match (lexer.tokens_partial)(lexer, meta.into(), max_lines) {
            Ok(request) => lexer.requests.push(request),
            Err(error) => gs.send_error(error, lexer.lang.file_type),
        };
    }
}

#[inline(always)]
pub fn sync_edits(lexer: &mut Lexer, action: &EditType, content: &mut [EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let (meta, change_events) = action.change_event(lexer.encode_position, lexer.char_lsp_pos, content);
    lexer.client.sync(lexer.uri.clone(), lexer.version, change_events)?;
    match lexer.meta.take() {
        Some(meta) => lexer.meta.replace(meta + meta),
        None => lexer.meta.replace(meta),
    };
    Ok(())
}

pub fn sync_edits_rev(lexer: &mut Lexer, action: &EditType, content: &mut [EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let (meta, change_events) = action.change_event_rev(lexer.encode_position, lexer.char_lsp_pos, content);
    lexer.client.sync(lexer.uri.clone(), lexer.version, change_events)?;
    match lexer.meta.take() {
        Some(meta) => lexer.meta.replace(meta + meta),
        None => lexer.meta.replace(meta),
    };
    Ok(())
}

#[inline(always)]
pub fn sync_edits_full(lexer: &mut Lexer, action: &EditType, content: &mut [EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let mut text = String::new();
    for editor_line in content.iter() {
        editor_line.push_content_to_buffer(&mut text);
        text.push('\n');
    }
    text.push('\n');
    lexer.client.full_sync(lexer.uri.clone(), lexer.version, text)?;
    match lexer.meta.take() {
        Some(meta) => lexer.meta.replace(meta + action.map_to_meta()),
        None => lexer.meta.replace(action.map_to_meta()),
    };
    Ok(())
}

pub fn sync_edits_full_rev(lexer: &mut Lexer, action: &EditType, content: &mut [EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let mut text = String::new();
    for editor_line in content.iter() {
        editor_line.push_content_to_buffer(&mut text);
        text.push('\n');
    }
    text.push('\n');
    lexer.client.full_sync(lexer.uri.clone(), lexer.version, text)?;
    match lexer.meta.take() {
        Some(meta) => lexer.meta.replace(meta + action.map_to_meta_rev()),
        None => lexer.meta.replace(action.map_to_meta_rev()),
    };
    Ok(())
}

#[inline(always)]
pub fn sync_edits_dead(_lexer: &mut Lexer, _action: &EditType, _content: &mut [EditorLine]) -> LSPResult<()> {
    Ok(())
}

#[inline(always)]
pub fn sync_edits_dead_rev(_lexer: &mut Lexer, _action: &EditType, _content: &mut [EditorLine]) -> LSPResult<()> {
    Ok(())
}

pub fn completable(lexer: &Lexer, char_idx: usize, line: &EditorLine) -> bool {
    !matches!(lexer.modal, Some(LSPModal::AutoComplete(..)))
        && !lexer.requests.iter().any(|req| matches!(req, LSPResponseType::Completion(..)))
        && lexer.lang.completable(line, char_idx)
}

pub fn get_autocomplete(lexer: &mut Lexer, c: CursorPosition, line: String, gs: &mut GlobalState) {
    match lexer.client.request_completions(lexer.uri.clone(), c).map(|id| LSPResponseType::Completion(id, line, c)) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn completable_dead(_lexer: &Lexer, _idx: usize, _line: &EditorLine) -> bool {
    false
}

pub fn get_autocomplete_dead(_: &mut Lexer, _: CursorPosition, _: String, _: &mut GlobalState) {}

pub fn tokens(lexer: &mut Lexer) -> LSPResult<LSPResponseType> {
    lexer.client.request_full_tokens(lexer.uri.clone()).map(LSPResponseType::Tokens)
}

pub fn tokens_dead(_: &mut Lexer) -> LSPResult<LSPResponseType> {
    Ok(LSPResponseType::Tokens(0))
}

pub fn tokens_partial(lexer: &mut Lexer, range: Range, max_lines: usize) -> LSPResult<LSPResponseType> {
    lexer
        .client
        .request_partial_tokens(lexer.uri.clone(), range)
        .map(|id| LSPResponseType::TokensPartial { id, max_lines })
}

pub fn tokens_partial_redirect(lexer: &mut Lexer, _: Range, _: usize) -> LSPResult<LSPResponseType> {
    tokens(lexer)
}

pub fn tokens_partial_dead(_: &mut Lexer, _: Range, _: usize) -> LSPResult<LSPResponseType> {
    Ok(LSPResponseType::TokensPartial { id: 0, max_lines: 0 })
}

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
