use crate::global_state::GlobalState;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::WidgetRef;
mod context;
mod diagnostics;
mod internal;
mod langs;
mod legend;
mod tokens;
use super::modal::LSPResponseType;
use crate::{lsp::LSPClient, syntax::Theme, workspace::actions::EditMetaData};
pub use context::LineBuilderContext;
use diagnostics::diagnostics_full;
pub use diagnostics::{Action, DiagnosticInfo, DiagnosticLine};
use internal::generic_line;
pub use langs::Lang;
use legend::Legend;
use lsp_types::{
    SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities, TextDocumentContentChangeEvent,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::path::Path;
use tokens::Tokens;

/// !the initial len of the line produced by init_buffer_with_line_number
/// !used by LineBuilder::format_with_info(..) -> ListItem - used to derive cursor and wrap
const INIT_BUF_SIZE: usize = 1;
const DIGIT_STYLE: Style = Style::new().fg(Color::DarkGray);

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
    legend: Legend,
    tokens: Tokens,
    diagnostic_processor: fn(&mut Self, Vec<(usize, DiagnosticLine)>),
}

impl LineBuilder {
    pub fn new(lang: Lang, content: &[String], gs: &mut GlobalState) -> Self {
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
    pub fn set_tokens(&mut self, tokens_res: SemanticTokensResult, content: &[String]) -> bool {
        if let SemanticTokensResult::Tokens(tokens) = tokens_res {
            self.tokens.tokens_reset(tokens.data, &self.legend, &self.lang, &self.theme, content);
        }
        !self.tokens.is_empty()
    }

    /// Process SemanticTokenRangeResult from LSP
    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult, content: &[String]) {
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
        content: &[String],
        client: &mut LSPClient,
    ) -> Option<LSPResponseType> {
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

    pub fn update_internals(
        &mut self,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[String],
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

    /// build styled line with diagnostic - reverts to native builder if tokens are not available
    pub fn build_line(
        &mut self,
        line_idx: usize,
        content: &str,
        line_number_offset: usize,
        buf: &mut Buffer,
        area: ratatui::prelude::Rect,
        ctx: &mut LineBuilderContext,
    ) {
        ctx.build_select_buffer(line_idx, content.len());
        if content.is_empty() {
            let mut buffer = init_buffer_with_line_number(line_idx, line_number_offset);
            if ctx.select_range.is_some() {
                buffer.push(Span::styled(" ", Style { bg: Some(self.theme.selected), ..Default::default() }));
            };
            return Line::from(ctx.format_with_info(line_idx, None, buffer)).render_ref(area, buf);
        }
        if !self.lsp_line(line_idx, line_number_offset, content, buf, area, ctx) {
            Line::from(generic_line(
                self,
                line_idx,
                content,
                ctx,
                init_buffer_with_line_number(line_idx, line_number_offset),
            ))
            .render_ref(area, buf);
        }
    }

    pub fn basic_line(&self, content: &str, ctx: &mut LineBuilderContext) -> Vec<Span<'static>> {
        let buffer = Vec::new();
        generic_line(self, usize::MAX, content, ctx, buffer)
    }

    pub fn split_line(
        &mut self,
        line_idx: usize,
        content: &str,
        line_number_offset: usize,
        ctx: &mut LineBuilderContext,
    ) -> Vec<Line<'static>> {
        todo!()
        // let mut buffer = self.build_line(line_idx, content, line_number_offset, ctx);
        // let padding = derive_wrap_digit_offset(buffer.first());
        // let mut lines = vec![Line::from(buffer.drain(..ctx.text_width + 1).collect::<Vec<_>>())];
        // while buffer.len() > ctx.text_width {
        // let mut line = vec![padding.clone()];
        // line.extend(buffer.drain(..ctx.text_width));
        // lines.push(Line::from(line));
        // }
        // buffer.insert(0, padding);
        // lines.push(Line::from(buffer));
        // lines
    }

    fn lsp_line(
        &mut self,
        line_idx: usize,
        max_digits: usize,
        content: &str,
        buf: &mut Buffer,
        area: Rect,
        ctx: &mut LineBuilderContext,
    ) -> bool {
        if ctx.select_range.is_none() && ctx.cursor.line != line_idx {
            return self.tokens.cached_render(line_idx, max_digits, content, buf, area);
        };
        if let Some(token_line) = self.tokens.get(line_idx) {
            let mut buffer = init_buffer_with_line_number(line_idx, max_digits);
            let mut style = Style::default();
            let mut remaining_word_len: usize = 0;
            let mut token_num = 0;
            for (char_idx, ch) in content.char_indices() {
                remaining_word_len = remaining_word_len.saturating_sub(1);
                if remaining_word_len == 0 {
                    match token_line.tokens.get(token_num) {
                        Some(token) if token.from == char_idx => {
                            remaining_word_len = token.len;
                            style.fg = Some(token.color);
                            token_num += 1;
                        }
                        _ => style.fg = None,
                    }
                }
                if matches!(&ctx.select_range, Some(range) if range.contains(&char_idx)) {
                    style.bg.replace(self.theme.selected);
                }
                buffer.push(Span::styled(ch.to_string(), ctx.brackets.map_style(ch, style)));
                style.add_modifier = Modifier::empty();
                style.bg = None;
            }
            Line::from(ctx.format_with_info(line_idx, Some(&token_line.diagnosics), buffer)).render_ref(area, buf);
            return true;
        }
        false
    }
}

fn derive_wrap_digit_offset(start_span: Option<&Span<'_>>) -> Span<'static> {
    if let Some(span) = start_span {
        let padding_len = span.content.len();
        return Span::raw((0..padding_len).map(|_| '.').collect::<String>());
    }
    Span::default()
}
