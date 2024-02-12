mod context;
mod diagnostics;
mod internal;
mod langs;
mod legend;
mod tokens;
use super::modal::LSPResponseType;
use crate::{lsp::LSPClient, syntax::Theme, workspace::actions::EditMetaData};
pub use context::LineBuilderContext;
pub use diagnostics::DiagnosticLine;
use diagnostics::{diagnostics_error, diagnostics_full};
use internal::generic_line;
use langs::Lang;
use legend::{ColorResult, Legend};
use lsp_types::{
    SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities, TextDocumentContentChangeEvent,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{collections::HashMap, path::Path};
use tokens::Tokens;

/// !the initial len of the line produced by init_buffer_with_line_number
/// !used by LineBuilder::format_with_info(..) -> ListItem - used to derive cursor and wrap
const INIT_BUF_SIZE: usize = 1;
const DIGIT_STYLE: Style = Style::new().fg(Color::Gray);

/// ! generates start with line number -> based on the produced vec len is the definition of INIT_BUF_SIZE
pub fn init_buffer_with_line_number(line_idx: usize, line_number_offset: usize) -> Vec<Span<'static>> {
    vec![Span::styled(
        format!("{: >1$} ", line_idx + 1, line_number_offset),
        DIGIT_STYLE,
    )]
}

/// Struct used to create styled maps
pub struct LineBuilder {
    pub theme: Theme,
    pub lang: Lang,
    file_was_saved: bool,
    legend: Legend,
    tokens: Tokens,
    diagnostics: HashMap<usize, DiagnosticLine>,
    diagnostic_processor: fn(&mut Self, Vec<(usize, DiagnosticLine)>),
}

impl LineBuilder {
    pub fn new(lang: Lang) -> Self {
        Self {
            theme: Theme::new(),
            lang,
            file_was_saved: true,
            legend: Legend::default(),
            tokens: Tokens::default(),
            diagnostics: HashMap::new(),
            diagnostic_processor: diagnostics_full,
        }
    }

    /// alternate diagnostic representation on save
    pub fn mark_saved(&mut self) {
        self.file_was_saved = true;
        self.diagnostic_processor = diagnostics_full;
    }

    /// Process Diagnostic notification from LSP
    pub fn set_diganostics(&mut self, diagnostics: Vec<(usize, DiagnosticLine)>) {
        self.diagnostics.clear();
        (self.diagnostic_processor)(self, diagnostics);
    }

    /// Process SemanticTokensResultFull from LSP
    pub fn set_tokens(&mut self, tokens_res: SemanticTokensResult) -> bool {
        if let SemanticTokensResult::Tokens(tokens) = tokens_res {
            self.tokens.tokens_reset(tokens.data);
        }
        !self.tokens.is_empty()
    }

    /// Process SemanticTokenRangeResult from LSP
    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult) {
        let tokens = match tokens {
            SemanticTokensRangeResult::Partial(data) => data.data,
            SemanticTokensRangeResult::Tokens(data) => data.data,
        };
        self.tokens.tokens_set(tokens);
    }

    /// Sync text edits with LSP
    pub fn collect_changes(
        &mut self,
        path: &Path,
        version: i32,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[String],
        client: &mut LSPClient,
    ) -> Option<LSPResponseType> {
        if self.file_was_saved {
            // remove warnings on change after save
            self.file_was_saved = false;
            self.diagnostic_processor = diagnostics_error;
            for (_, data) in self.diagnostics.iter_mut() {
                data.drop_non_errs();
            }
        }
        if self.tokens.is_empty() {
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

    /// gets possible actions from diagnostic data
    pub fn collect_actions(&self, line: usize) -> Option<Vec<String>> {
        self.diagnostics.get(&line).and_then(|d_line| d_line.collect_actions())
    }

    /// Maps token styles
    pub fn map_styles(&mut self, tokens_res: &Option<SemanticTokensServerCapabilities>) {
        if let Some(capabilities) = tokens_res {
            self.legend.map_styles(&self.lang.file_type, &self.theme, capabilities)
        }
    }

    /// build styled line with diagnostic - reverts to native builder if tokens are not available
    pub fn build_line(
        &self,
        line_idx: usize,
        content: &str,
        line_number_offset: usize,
        ctx: &mut LineBuilderContext,
    ) -> Vec<Span<'static>> {
        ctx.build_select_buffer(line_idx, content.len());
        if content.is_empty() {
            let mut buffer = init_buffer_with_line_number(line_idx, line_number_offset);
            if ctx.select_range.is_some() {
                buffer.push(Span::styled(" ", Style { bg: Some(self.theme.selected), ..Default::default() }));
            };
            return ctx.format_with_info(line_idx, None, buffer);
        }
        match self.lsp_line(line_idx, content, line_number_offset, ctx) {
            Some(line) => line,
            None => {
                generic_line(self, line_idx, content, ctx, init_buffer_with_line_number(line_idx, line_number_offset))
            }
        }
    }

    pub fn split_line(
        &self,
        line_idx: usize,
        content: &str,
        split_len: usize,
        line_number_offset: usize,
        ctx: &mut LineBuilderContext,
    ) -> Vec<Line<'static>> {
        let mut buffer = self.build_line(line_idx, content, line_number_offset, ctx);
        let padding = derive_wrap_digit_offset(buffer.first());
        let mut lines = vec![Line::from(buffer.drain(..split_len).collect::<Vec<_>>())];
        let expected_width = split_len - 1;
        while buffer.len() > expected_width {
            let mut line = vec![padding.clone()];
            line.extend(buffer.drain(..expected_width));
            lines.push(Line::from(line));
        }
        buffer.insert(0, padding);
        lines.push(Line::from(buffer));
        lines
    }

    fn lsp_line(
        &self,
        line_idx: usize,
        content: &str,
        max_digits: usize,
        ctx: &mut LineBuilderContext,
    ) -> Option<Vec<Span<'static>>> {
        let token_line = self.tokens.get(line_idx)?;
        let mut buffer = init_buffer_with_line_number(line_idx, max_digits);
        let mut style = Style::default();
        let mut remaining_word_len: usize = 0;
        let mut token_num = 0;
        let diagnostic = self.diagnostics.get(&line_idx);
        for (char_idx, ch) in content.char_indices() {
            remaining_word_len = remaining_word_len.saturating_sub(1);
            if remaining_word_len == 0 {
                match token_line.get(token_num) {
                    Some(token) if token.from == char_idx => {
                        remaining_word_len = token.len;
                        style.fg = Some(match self.legend.get_color(token.token_type, &self.theme) {
                            ColorResult::Final(color) => color,
                            ColorResult::KeyWord => match content.get(char_idx..(char_idx + remaining_word_len)) {
                                Some(slice) => self.handle_keywords(slice),
                                None => self.theme.key_words,
                            },
                        });
                        token_num += 1;
                    }
                    _ => style.fg = None,
                }
            }
            if let Some(diagnostic) = diagnostic {
                diagnostic.set_diagnostic_style(char_idx, &mut style);
            }
            if matches!(&ctx.select_range, Some(range) if range.contains(&char_idx)) {
                style.bg.replace(self.theme.selected);
            }
            ctx.brackets.map_style(ch, &mut style);
            buffer.push(Span::styled(ch.to_string(), style));
            style.add_modifier = Modifier::empty();
            style.bg = None;
        }
        Some(ctx.format_with_info(line_idx, diagnostic, buffer))
    }

    fn handle_keywords(&self, word: &str) -> Color {
        if self.lang.frow_control.contains(&word) {
            return self.theme.flow_control;
        }
        self.theme.key_words
    }
}

fn derive_wrap_digit_offset(start_span: Option<&Span<'_>>) -> Span<'static> {
    if let Some(span) = start_span {
        let padding_len = span.content.len();
        return Span::raw((0..padding_len).map(|_| ' ').collect::<String>());
    }
    Span::default()
}
