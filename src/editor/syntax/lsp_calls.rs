use crate::{
    actions::{Action, EditMetaData},
    configs::Theme,
    cursor::CursorPosition,
    editor::{
        syntax::{
            set_diganostics,
            tokens::{set_tokens, set_tokens_partial},
            Encoding, Lexer,
        },
        Editor,
    },
    editor_line::EditorLine,
    global_state::{GlobalState, IdiomEvent},
    lsp::{LSPClient, LSPResponse, LSPResult},
    popups::popups_tree::refrence_selector,
};
use core::str::FromStr;
use lsp_types::{
    OneOf, Range, SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    TextDocumentContentChangeEvent, Uri, WorkspaceEdit,
};
use std::path::Path;

/// maps LSP state without runtime checks
#[inline]
pub fn map_lsp(lexer: &mut Lexer, client: LSPClient, theme: &Theme) {
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
        lexer.legend.map_styles(lexer.lang.file_type, theme, tc);
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

    // document syncing - only incremental is handled, in case of full wrapper of stdout will handle full sync in separate thread
    if client.capabilities.text_document_sync.is_some() {
        lexer.sync_tokens = sync_tokens;
        lexer.sync_changes = sync_changes;
        lexer.sync = sync_edits;
        lexer.sync_rev = sync_edits_rev;
    } else {
        lexer.sync_tokens = sync_tokens_dead;
        lexer.sync_changes = sync_changes_dead;
        lexer.sync = sync_edits_dead;
        lexer.sync_rev = sync_edits_rev_dead;
    }

    match client.capabilities.position_encoding.as_ref().map(|encode| encode.as_str()) {
        Some("utf-8") => {
            lexer.encoding = Encoding::utf8();
        }
        Some("utf-32") => {
            lexer.encoding = Encoding::utf32();
        }
        _ => {
            lexer.encoding = Encoding::utf16();
        }
    }

    if let Some(formatter) = client.capabilities.document_formatting_provider.as_ref() {
        match formatter {
            OneOf::Left(true) | OneOf::Right(..) => lexer.formatting = formatting,
            _ => lexer.formatting = formatting_dead,
        }
    }

    lexer.client = client;
}

pub fn remove_lsp(lexer: &mut Lexer) {
    lexer.lsp = false;
    lexer.client = LSPClient::placeholder();
    lexer.context = context_local;
    lexer.completable = completable_disable;
    lexer.autocomplete = get_autocomplete_dead;
    lexer.tokens = tokens_dead;
    lexer.tokens_partial = tokens_partial_dead;
    lexer.references = info_position_dead;
    lexer.definitions = info_position_dead;
    lexer.declarations = info_position_dead;
    lexer.hover = info_position_dead;
    lexer.signatures = info_position_dead;
    lexer.sync = sync_edits_dead;
    lexer.sync_rev = sync_edits_rev_dead;
    lexer.encoding = Encoding::utf32();
}

pub fn context_local(_: &mut Editor, _: &mut GlobalState) {}

pub fn context(editor: &mut Editor, gs: &mut GlobalState) {
    if editor.lexer.question_lsp && editor.lexer.client.is_closed() {
        gs.error("LSP failure ...");
        gs.event.push(crate::global_state::IdiomEvent::CheckLSP(editor.file_type));
        return;
    }

    handle_diagnosticts(editor, gs);
    handle_responses(editor, gs);

    if let Some(meta) = editor.lexer.meta.take() {
        let max_lines = (meta.start_line + meta.to) - 1;
        if max_lines >= editor.content.len() {
            return;
        }
        match (editor.lexer.tokens_partial)(&mut editor.lexer, meta.into(), max_lines) {
            Ok(request) => editor.lexer.requests.push(request),
            Err(error) => gs.send_error(error, editor.lexer.lang.file_type),
        };
    }
}

/// ignores partial tokens
pub fn context_awaiting_tokens(editor: &mut Editor, gs: &mut GlobalState) {
    if editor.lexer.question_lsp && editor.lexer.client.is_closed() {
        gs.error("LSP failure ...");
        gs.event.push(crate::global_state::IdiomEvent::CheckLSP(editor.file_type));
        return;
    }

    handle_diagnosticts(editor, gs);
    handle_responses(editor, gs);
}

#[inline(always)]
fn handle_responses(editor: &mut Editor, gs: &mut GlobalState) {
    let Some(mut responses) = editor.lexer.client.get_responses() else {
        return;
    };

    let modal = &mut editor.modal;
    let content = &mut editor.content;

    for id in std::mem::take(&mut editor.lexer.requests) {
        let Some(response) = responses.remove(&id) else {
            editor.lexer.requests.push(id);
            continue;
        };
        match response {
            LSPResponse::Completion(completions, line_idx) => {
                editor.lexer.completion_cache = None;
                if editor.cursor.line == line_idx {
                    let line = content[line_idx].as_str();
                    modal.auto_complete(completions, line, editor.cursor.get_position(), &gs.matcher);
                }
            }
            LSPResponse::Hover(hover) => modal.map_hover(hover, &gs.theme),
            LSPResponse::SignatureHelp(signature) => modal.map_signatures(signature, &gs.theme),
            LSPResponse::Renames(workspace_edit) => gs.event.push(workspace_edit.into()),
            LSPResponse::Formatting { edits, save } => {
                if save {
                    gs.event.push(crate::global_state::IdiomEvent::Save)
                };
                gs.event.push(WorkspaceEdit::new([(editor.lexer.uri.clone(), edits)].into_iter().collect()).into());
            }
            LSPResponse::Tokens(tokens) => {
                match tokens {
                    SemanticTokensResult::Partial(data) => set_tokens(data.data, &editor.lexer.legend, content),
                    SemanticTokensResult::Tokens(data) => {
                        set_tokens(data.data, &editor.lexer.legend, content);
                        gs.success("LSP tokens mapped! Refresh UI to remove artifacts (default F5)");
                    }
                };
                editor.lexer.context = context;
            }
            LSPResponse::TokensPartial { result, max_lines } => {
                let tokens = match result {
                    SemanticTokensRangeResult::Partial(data) => data.data,
                    SemanticTokensRangeResult::Tokens(data) => data.data,
                };
                set_tokens_partial(tokens, max_lines, &editor.lexer.legend, content);
            }
            LSPResponse::References(locations) => {
                if let Some(mut locations) = locations {
                    if locations.len() == 1 {
                        gs.event.push(locations.remove(0).into());
                    } else {
                        gs.event.push(refrence_selector(locations).into())
                    }
                }
            }
            LSPResponse::Declaration(declaration) => gs.try_tree_event(declaration),
            LSPResponse::Definition(definition) => gs.try_tree_event(definition),
            LSPResponse::Error(text) => gs.error(text),
            LSPResponse::Empty => (),
        }
    }
}

#[inline(always)]
fn handle_diagnosticts(editor: &mut Editor, gs: &mut GlobalState) {
    let (editor_diagnostics, tree_diagnostics) = editor.lexer.client.get_diagnostics(&editor.lexer.uri);
    if let Some(diagnostics) = editor_diagnostics {
        set_diganostics(&mut editor.content, diagnostics);
        editor.modal.cleanr_render_cache(); // force rebuild
    }

    if let Some(tree_diagnostics) = tree_diagnostics {
        gs.event.push(IdiomEvent::TreeDiagnostics(tree_diagnostics));
    }
}

// HANDLERS

pub fn sync_changes(lexer: &mut Lexer, change_events: Vec<TextDocumentContentChangeEvent>) -> LSPResult<()> {
    lexer.version += 1;
    lexer.client.sync(lexer.uri.clone(), lexer.version, change_events)
}

pub fn sync_edits(lexer: &mut Lexer, action: &Action, content: &[EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let (meta, change_events) = action.change_event(lexer.encoding.encode_position, lexer.encoding.char_len, content);
    lexer.client.sync(lexer.uri.clone(), lexer.version, change_events)?;
    match lexer.meta.take() {
        Some(existing_meta) => lexer.meta.replace(existing_meta + meta),
        None => lexer.meta.replace(meta),
    };
    Ok(())
}

pub fn sync_edits_rev(lexer: &mut Lexer, action: &Action, content: &[EditorLine]) -> LSPResult<()> {
    lexer.version += 1;
    let (meta, change_events) =
        action.change_event_rev(lexer.encoding.encode_position, lexer.encoding.char_len, content);
    lexer.client.sync(lexer.uri.clone(), lexer.version, change_events)?;
    match lexer.meta.take() {
        Some(existing_meta) => lexer.meta.replace(existing_meta + meta),
        None => lexer.meta.replace(meta),
    };
    Ok(())
}

pub fn sync_tokens(lexer: &mut Lexer, meta: EditMetaData) {
    match lexer.meta.take() {
        Some(existing_meta) => lexer.meta.replace(existing_meta + meta),
        None => lexer.meta.replace(meta),
    };
}

#[inline(always)]
pub fn sync_tokens_dead(_lexer: &mut Lexer, _meta: EditMetaData) {}

#[inline(always)]
pub fn sync_changes_dead(_lexer: &mut Lexer, _change_events: Vec<TextDocumentContentChangeEvent>) -> LSPResult<()> {
    Ok(())
}

#[inline(always)]
pub fn sync_edits_dead(_lexer: &mut Lexer, _action: &Action, _content: &[EditorLine]) -> LSPResult<()> {
    Ok(())
}

#[inline(always)]
pub fn sync_edits_rev_dead(_lexer: &mut Lexer, _action: &Action, _content: &[EditorLine]) -> LSPResult<()> {
    Ok(())
}

pub fn completable(lexer: &Lexer, char_idx: usize, line: &EditorLine) -> bool {
    lexer.lang.completable(line, char_idx)
}

pub fn completable_disable(_: &Lexer, _: usize, _: &EditorLine) -> bool {
    false
}

pub fn get_autocomplete(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_completions(lexer.uri.clone(), c) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn get_autocomplete_dead(_: &mut Lexer, _: CursorPosition, _: &mut GlobalState) {}

pub fn tokens(lexer: &mut Lexer) -> LSPResult<i64> {
    lexer.context = context_awaiting_tokens;
    lexer.client.request_full_tokens(lexer.uri.clone())
}

pub fn tokens_dead(_: &mut Lexer) -> LSPResult<i64> {
    Ok(0)
}

pub fn tokens_partial(lexer: &mut Lexer, range: Range, max_lines: usize) -> LSPResult<i64> {
    lexer.client.request_partial_tokens(lexer.uri.clone(), range, max_lines)
}

pub fn tokens_partial_redirect(lexer: &mut Lexer, _: Range, _: usize) -> LSPResult<i64> {
    tokens(lexer)
}

pub fn tokens_partial_dead(_: &mut Lexer, _: Range, _: usize) -> LSPResult<i64> {
    Ok(0)
}

pub fn formatting(lexer: &mut Lexer, indent: usize, save: bool, gs: &mut GlobalState) {
    match lexer.client.formatting(lexer.uri.clone(), indent, save) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn formatting_dead(_: &mut Lexer, _: usize, _: bool, _: &mut GlobalState) {}

pub fn info_position_dead(_: &mut Lexer, _: CursorPosition, _: &mut GlobalState) {}

pub fn references(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_references(lexer.uri.clone(), c) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn definitions(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_definitions(lexer.uri.clone(), c) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn declarations(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_declarations(lexer.uri.clone(), c) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn hover(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_hover(lexer.uri.clone(), c) {
        Ok(request) => lexer.requests.push(request),
        Err(err) => gs.send_error(err, lexer.lang.file_type),
    }
}

pub fn signatures(lexer: &mut Lexer, c: CursorPosition, gs: &mut GlobalState) {
    match lexer.client.request_signitures(lexer.uri.clone(), c) {
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

#[inline(always)]
pub fn as_url(path: &Path) -> Uri {
    Uri::from_str(format!("file://{}", path.display()).as_str()).expect("Path should always be parsable!")
}
