mod brackets;
mod diagnostics;
mod internal;
mod langs;
mod legend;
mod tokens;

use tokens::Tokens;

use super::modal::LSPResponseType;
use crate::{
    lsp::LSPClient,
    syntax::Theme,
    workspace::{actions::EditMetaData, cursor::Cursor, CursorPosition},
};
use brackets::BracketColors;
use diagnostics::{diagnostics_error, diagnostics_full, DiagnosticLine};
use internal::generic_line;
use langs::Lang;
use legend::{ColorResult, Legend};

use lsp_types::{
    PublishDiagnosticsParams, SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    TextDocumentContentChangeEvent,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use std::{cmp::Ordering, collections::HashMap, ops::Range, path::Path};

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
    pub text_width: usize,
    select: Option<(CursorPosition, CursorPosition)>,
    select_range: Option<Range<usize>>,
    file_was_saved: bool,
    legend: Legend,
    tokens: Tokens,
    cursor: CursorPosition,
    brackets: BracketColors,
    diagnostics: HashMap<usize, DiagnosticLine>,
    diagnostic_processor: fn(&mut Self, PublishDiagnosticsParams),
}

impl LineBuilder {
    pub fn new(lang: Lang) -> Self {
        Self {
            theme: Theme::new(),
            lang,
            text_width: 0,
            select: None,
            select_range: None,
            file_was_saved: true,
            legend: Legend::default(),
            tokens: Tokens::default(),
            cursor: CursorPosition::default(),
            brackets: BracketColors::default(),
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
    pub fn set_diganostics(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.clear();
        (self.diagnostic_processor)(self, params);
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

    /// reset linebuilder after processing whole frame done on context exchange with editro
    pub fn reset(&mut self, cursor: &Cursor) {
        self.cursor = cursor.position();
        self.select = cursor.select_get();
        self.brackets.reset();
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
    pub fn build_line<'a>(&mut self, line_idx: usize, content: &'a str, line_number_offset: usize) -> ListItem<'a> {
        self.build_select_buffer(line_idx, content.len());
        if content.is_empty() {
            let mut buffer = init_buffer_with_line_number(line_idx, line_number_offset);
            if self.select_range.is_some() {
                buffer.push(Span::styled(" ", Style { bg: Some(self.theme.selected), ..Default::default() }));
            };
            return self.format_with_info(line_idx, None, buffer);
        }
        match self.process_tokens(line_idx, content, line_number_offset) {
            Some(line) => line,
            None => generic_line(self, line_idx, content, init_buffer_with_line_number(line_idx, line_number_offset)),
        }
    }

    fn process_tokens<'a>(&mut self, line_idx: usize, content: &'a str, max_digits: usize) -> Option<ListItem<'a>> {
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
            self.set_diagnostic_style(char_idx, &mut style, diagnostic);
            self.set_select(&mut style, &char_idx);
            self.brackets.map_style(ch, &mut style);
            buffer.push(Span::styled(ch.to_string(), style));
            style.add_modifier = Modifier::empty();
            style.bg = None;
        }
        Some(self.format_with_info(line_idx, diagnostic, buffer))
    }

    fn format_with_info<'a>(
        &self,
        line_idx: usize,
        diagnostic: Option<&DiagnosticLine>,
        mut buffer: Vec<Span<'a>>,
    ) -> ListItem<'a> {
        // set cursor without the normal API
        if line_idx == self.cursor.line {
            let expected = self.cursor.char + INIT_BUF_SIZE;
            if buffer.len() > expected {
                buffer[self.cursor.char + INIT_BUF_SIZE].style.add_modifier = Modifier::REVERSED;
            } else {
                buffer.push(Span::styled(" ", Style { add_modifier: Modifier::REVERSED, ..Default::default() }))
            }
        };

        if buffer.len() > self.text_width {
            let padding = derive_wrap_digit_offset(buffer.first());
            let mut lines = vec![Line::from(buffer.drain(..self.text_width).collect::<Vec<_>>())];
            let expected_width = self.text_width - 1;
            while buffer.len() > expected_width {
                let mut line = vec![padding.clone()];
                line.extend(buffer.drain(..expected_width));
                lines.push(Line::from(line));
            }
            buffer.insert(0, padding);
            if let Some(diagnostic) = diagnostic {
                buffer.extend(diagnostic.data.iter().map(|d| d.span.clone()));
            }
            lines.push(Line::from(buffer));
            ListItem::from(lines)
        } else {
            if let Some(diagnostic) = diagnostic {
                buffer.extend(diagnostic.data.iter().map(|d| d.span.clone()));
            }
            ListItem::from(Line::from(buffer))
        }
    }

    fn build_select_buffer(&mut self, at_line: usize, max_len: usize) {
        self.select_range = self.select.and_then(|(from, to)| match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
            (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
            (Ordering::Less, Ordering::Less) => Some(0..max_len),
            (Ordering::Equal, Ordering::Equal) => Some(from.char..to.char),
            (Ordering::Equal, ..) => Some(from.char..max_len),
            (.., Ordering::Equal) => Some(0..to.char),
        });
    }

    fn set_diagnostic_style(&self, idx: usize, style: &mut Style, diagnostic: Option<&DiagnosticLine>) {
        if let Some(color) = diagnostic.and_then(|d| d.check_ranges(&idx)) {
            style.add_modifier = style.add_modifier.union(Modifier::UNDERLINED);
            style.underline_color.replace(color);
        }
    }

    fn set_select(&self, style: &mut Style, idx: &usize) {
        if matches!(&self.select_range, Some(range) if range.contains(idx)) {
            style.bg.replace(self.theme.selected);
        }
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
