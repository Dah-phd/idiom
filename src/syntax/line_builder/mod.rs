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
    workspace::{actions::EditMetaData, CursorPosition},
};
use brackets::BracketColors;
use diagnostics::{diagnostics_error, diagnostics_full, DiagnosticLines};
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
use std::{collections::HashMap, ops::Range, path::Path};

/// !the initial len of the line produced by init_buffer_with_line_number
/// !used by LineBuilder::format_with_info(..) -> ListItem - used to derive cursor and wrap
const INIT_BUF_SIZE: usize = 1;
const DIGIT_STYLE: Style = Style::new().fg(Color::Gray);

/// ! generates start with line number -> based on the produced vec len is the definition of INIT_BUF_SIZE
pub fn init_buffer_with_line_number(line_idx: usize, max_digits: usize) -> Vec<Span<'static>> {
    vec![Span::styled(format!("{: >1$} ", line_idx + 1, max_digits), DIGIT_STYLE)]
}

/// Struct used to create styled maps
pub struct LineBuilder {
    pub theme: Theme,
    pub lang: Lang,
    pub text_width: usize,
    select_range: Option<Range<usize>>,
    legend: Legend,
    tokens: Tokens,
    cursor: CursorPosition,
    brackets: BracketColors,
    diagnostics: HashMap<usize, DiagnosticLines>,
    diagnostic_processor: fn(&mut Self, PublishDiagnosticsParams),
    file_was_saved: bool,
}

impl LineBuilder {
    pub fn new(lang: Lang) -> Self {
        Self {
            theme: Theme::new(),
            lang,
            text_width: 0,
            select_range: None,
            legend: Legend::default(),
            tokens: Tokens::default(),
            cursor: CursorPosition::default(),
            brackets: BracketColors::default(),
            diagnostics: HashMap::new(),
            diagnostic_processor: diagnostics_full,
            file_was_saved: true,
        }
    }

    pub fn mark_saved(&mut self) {
        self.file_was_saved = true;
        self.diagnostic_processor = diagnostics_full;
    }

    pub fn set_diganostics(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.clear();
        (self.diagnostic_processor)(self, params);
    }

    pub fn set_tokens(&mut self, tokens_res: SemanticTokensResult) -> bool {
        if let SemanticTokensResult::Tokens(tokens) = tokens_res {
            self.tokens.tokens_reset(tokens.data);
        }
        !self.tokens.is_empty()
    }

    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult) {
        let tokens = match tokens {
            SemanticTokensRangeResult::Partial(data) => data.data,
            SemanticTokensRangeResult::Tokens(data) => data.data,
        };
        self.tokens.tokens_set(tokens);
    }

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

    pub fn reset(&mut self, c: CursorPosition) {
        self.cursor = c;
        self.brackets.reset();
    }

    pub fn build_line<'a>(
        &mut self,
        line_idx: usize,
        select: Option<Range<usize>>,
        content: &'a str,
        max_digits: usize,
    ) -> ListItem<'a> {
        if content.is_empty() {
            let mut buffer = init_buffer_with_line_number(line_idx, max_digits);
            if select.is_some() {
                buffer.push(Span::styled(" ", Style { bg: Some(self.theme.selected), ..Default::default() }));
            };
            return self.format_with_info(line_idx, None, buffer);
        }
        self.select_range = select;
        if let Some(line) = self.process_tokens(line_idx, content, max_digits) {
            line
        } else {
            generic_line(self, line_idx, content, init_buffer_with_line_number(line_idx, max_digits))
        }
    }

    pub fn process_tokens<'a>(&mut self, line_idx: usize, content: &'a str, max_digits: usize) -> Option<ListItem<'a>> {
        let token_line = self.tokens.get(line_idx)?;
        let mut buffer = init_buffer_with_line_number(line_idx, max_digits);
        let mut style = Style::default();
        let mut len: usize = 0;
        let mut token_num = 0;
        let diagnostic = self.diagnostics.get(&line_idx);
        for (char_idx, ch) in content.char_indices() {
            len = len.saturating_sub(1);
            if len == 0 {
                match token_line.get(token_num) {
                    Some(token) if token.from == char_idx => {
                        len = token.len;
                        style.fg = Some(match self.legend.get_color(token.token_type, &self.theme) {
                            ColorResult::Final(color) => color,
                            ColorResult::KeyWord => match content.get(char_idx..(char_idx + len)) {
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
        diagnostic: Option<&DiagnosticLines>,
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
            let mut lines = Vec::new();
            while buffer.len() > self.text_width {
                let mut line = buffer.drain(..self.text_width).collect::<Vec<_>>();
                line.push(Span::raw("\n"));
                lines.push(Line::from(line));
            }
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

    fn set_diagnostic_style(&self, idx: usize, style: &mut Style, diagnostic: Option<&DiagnosticLines>) {
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

    pub fn map_styles(&mut self, tokens_res: &Option<SemanticTokensServerCapabilities>) {
        if let Some(capabilities) = tokens_res {
            self.legend.map_styles(&self.lang.file_type, &self.theme, capabilities)
        }
    }
}
