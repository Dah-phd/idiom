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
        idx: usize,
        select: Option<Range<usize>>,
        content: &'a str,
        mut init: Vec<Span<'a>>,
    ) -> ListItem<'a> {
        if content.is_empty() {
            if select.is_some() {
                init.push(Span::styled(" ", Style { bg: Some(self.theme.selected), ..Default::default() }));
            };
            return self.format_with_info(idx, init.len(), None, init);
        }
        self.select_range = select;
        if let Some(line) = self.process_tokens(idx, content, init.clone()) {
            line
        } else {
            generic_line(self, idx, content, init)
        }
    }

    pub fn process_tokens<'a>(
        &mut self,
        line_idx: usize,
        content: &'a str,
        mut spans: Vec<Span<'a>>,
    ) -> Option<ListItem<'a>> {
        let mut style = Style { fg: Some(Color::White), ..Default::default() };
        let mut len: u32 = 0;
        let mut token_num = 0;
        let offset = spans.len();
        let token_line = self.tokens.get(line_idx)?;
        let diagnostic = self.diagnostics.get(&line_idx);
        for (idx, ch) in content.char_indices() {
            len = len.saturating_sub(1);
            if len == 0 {
                if let Some(t) = token_line.get(token_num) {
                    if t.from == idx {
                        len = t.len;
                        style.fg = Some(match self.legend.get_color(t.token_type, &self.theme) {
                            ColorResult::Final(color) => color,
                            ColorResult::KeyWord => {
                                if content.len() > idx + (len as usize) {
                                    self.handle_keywords(&content[idx..(idx + len as usize)])
                                } else {
                                    self.theme.key_words
                                }
                            }
                        });
                        token_num += 1;
                    } else {
                        style.fg.replace(Color::default());
                    }
                } else {
                    style.fg.replace(Color::default());
                }
            }
            self.set_diagnostic_style(idx, &mut style, diagnostic);
            self.set_select(&mut style, &idx);
            self.brackets.map_style(ch, &mut style);
            spans.push(Span::styled(ch.to_string(), style));
            style.add_modifier = Modifier::empty();
            style.bg = None;
        }
        Some(self.format_with_info(line_idx, offset, diagnostic, spans))
    }

    fn format_with_info<'a>(
        &self,
        line_idx: usize,
        offset: usize,
        diagnostic: Option<&DiagnosticLines>,
        mut buffer: Vec<Span<'a>>,
    ) -> ListItem<'a> {
        // set cursor without the normal API
        if line_idx == self.cursor.line {
            let expected = self.cursor.char + offset;
            if buffer.len() > expected {
                buffer[self.cursor.char + offset].style.add_modifier = Modifier::REVERSED;
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

pub struct Token {
    from: usize,
    len: u32,
    token_type: usize,
}
