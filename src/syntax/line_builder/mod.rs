mod context;
mod diagnostics;
mod langs;
mod legend;
pub mod tokens;

use crate::{
    global_state::GlobalState,
    lsp::LSPClient,
    syntax::{modal::LSPResponseType, Theme},
    workspace::{actions::EditMetaData, line::Line},
};
pub use context::LineBuilderContext;
use diagnostics::diagnostics_full;
pub use diagnostics::{Action, DiagnosticInfo, DiagnosticLine};
pub use langs::Lang;
use legend::Legend;
use lsp_types::{
    SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities, TextDocumentContentChangeEvent,
};
use ratatui::layout::Rect;
use std::{io::Stdout, path::Path};
use tokens::{set_tokens, Tokens};

/// Struct used to create styled maps
pub struct LineBuilder {
    pub theme: Theme,
    pub lang: Lang,
    legend: Legend,
    tokens: Tokens,
    diagnostic_processor: fn(&mut Self, Vec<(usize, DiagnosticLine)>),
}

impl LineBuilder {
    pub fn new(lang: Lang, content: &[impl Line], gs: &mut GlobalState) -> Self {
        let theme = gs.unwrap_default_result(Theme::new(), "theme.json: ");
        Self {
            tokens: Tokens::new(content, &lang, &theme),
            theme,
            lang,
            legend: Legend::default(),
            diagnostic_processor: diagnostics_full,
        }
    }

    /// alternate diagnostic representation on save
    pub fn mark_saved(&mut self) {
        self.diagnostic_processor = diagnostics_full;
    }

    /// Process Diagnostic notification from LSP
    pub fn set_diganostics(&mut self, diagnostics: Vec<(usize, DiagnosticLine)>) {
        (self.diagnostic_processor)(self, diagnostics);
    }

    /// Process SemanticTokensResultFull from LSP
    pub fn set_tokens(&mut self, tokens_res: SemanticTokensResult, content: &mut Vec<impl Line>) -> bool {
        if let SemanticTokensResult::Tokens(tokens) = tokens_res {
            if tokens.data.is_empty() {
                return false;
            }
            self.tokens.to_lsp();
            set_tokens(tokens.data, &self.legend, &self.lang, &self.theme, content);
            // self.tokens.tokens_reset_(tokens.data, &self.legend, &self.lang, &self.theme, content);
        }
        self.tokens.are_from_lsp()
    }

    /// Process SemanticTokenRangeResult from LSP
    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult, content: &[impl Line]) {
        let tokens = match tokens {
            SemanticTokensRangeResult::Partial(data) => data.data,
            SemanticTokensRangeResult::Tokens(data) => data.data,
        };
        self.tokens.tokens_set(tokens, &self.legend, &self.lang, &self.theme, content);
    }

    /// Sync text edits with LSP
    pub fn collect_changes(
        &mut self,
        path: &Path,
        version: i32,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[impl Line],
        client: &mut LSPClient,
    ) -> Option<LSPResponseType> {
        if self.tokens.are_from_lsp() {
            // ensures that there is fully mapped tokens before doing normal processing
            client.file_did_change(path, version, events.drain(..).map(|(_, edit)| edit).collect()).ok()?;
            return client.request_full_tokens(path).map(LSPResponseType::Tokens);
        }
        match events.len() {
            0 => None,
            1 => {
                let (meta, edit) = events.remove(0);
                self.tokens.map_meta_data(meta);
                client.file_did_change(path, version, vec![edit]).ok()?;
                client.request_partial_tokens(path, meta.build_range(content)?).map(LSPResponseType::TokensPartial)
            }
            _ => {
                let edits = events
                    .drain(..)
                    .map(|(meta, edit)| {
                        self.tokens.map_meta_data(meta);
                        edit
                    })
                    .collect::<Vec<_>>();
                client.file_did_change(path, version, edits).ok()?;
                client.request_full_tokens(path).map(LSPResponseType::Tokens)
            }
        }
    }

    pub fn update_internals(
        &mut self,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[impl Line],
    ) {
        match events.len() {
            0 => {}
            1 => {
                let (meta, _edit) = events.remove(0);
                self.tokens.map_meta_internal(meta, content, &self.lang, &self.theme);
            }
            _ => {
                self.tokens.rebuild_internals(content, &self.lang, &self.theme);
                events.clear();
            }
        }
    }

    /// gets possible actions from diagnostic data
    pub fn collect_diagnostic_info(&self, line_idx: usize) -> Option<DiagnosticInfo> {
        Some(self.tokens.diagnostic_info(line_idx, &self.lang))
    }

    /// Maps token styles
    pub fn map_styles(&mut self, tokens_res: &Option<SemanticTokensServerCapabilities>) {
        if let Some(capabilities) = tokens_res {
            self.legend.map_styles(&self.lang.file_type, &self.theme, capabilities)
        }
    }

    pub fn build_line(
        &mut self,
        line_idx: usize,
        max_digits: usize,
        content: &str,
        area: Rect,
        writer: &mut Stdout,
        ctx: &mut LineBuilderContext,
    ) -> std::io::Result<()> {
        ctx.build_select_buffer(line_idx, content.len());
        if ctx.cursor.line == line_idx {
            return self.tokens.get_or_create_line(line_idx).render(line_idx, max_digits, content, area, writer);
        };
        if ctx.select_range.is_some() {
            return self.tokens.get_or_create_line(line_idx).render_select(line_idx, max_digits, content, area, writer);
        };
        self.tokens.get_or_create_line(line_idx).fast_render(line_idx, max_digits, content, area, writer)
    }
}
