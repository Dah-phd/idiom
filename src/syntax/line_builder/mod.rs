mod brackets;
mod diagnostics;
mod internal;
mod langs;
mod legend;
use super::modal::LSPResponseType;
use crate::{lsp::LSPClient, syntax::Theme, workspace::actions::EditMetaData};
use brackets::BracketColors;
use diagnostics::{diagnostics_error, diagnostics_full, DiagnosticLines};
use internal::SpansBuffer;
use langs::Lang;
use legend::{ColorResult, Legend};

use lsp_types::{
    PublishDiagnosticsParams, SemanticTokensRangeResult, SemanticTokensResult, SemanticTokensServerCapabilities,
    TextDocumentContentChangeEvent,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{collections::HashMap, ops::Range, path::Path};

#[derive(Debug)]
pub struct LineBuilder {
    pub tokens: Vec<Vec<Token>>,
    pub legend: Legend,
    pub theme: Theme,
    pub lang: Lang,
    diagnostic_processor: fn(&mut Self, PublishDiagnosticsParams),
    brackets: BracketColors,
    diagnostics: HashMap<usize, DiagnosticLines>,
    pub select_range: Option<Range<usize>>,
    pub file_was_saved: bool,
    ignores: Vec<usize>,
}

impl LineBuilder {
    pub fn new(lang: Lang) -> Self {
        Self {
            tokens: Vec::new(),
            legend: Legend::default(),
            theme: Theme::new(),
            lang,
            diagnostic_processor: diagnostics_full,
            brackets: BracketColors::default(),
            diagnostics: HashMap::new(),
            select_range: None,
            file_was_saved: true,
            ignores: Vec::new(),
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
        let mut inner_token = Vec::new();
        let mut from = 0;
        self.tokens.clear();
        if let SemanticTokensResult::Tokens(tkns) = tokens_res {
            for tkn in tkns.data {
                for _ in 0..tkn.delta_line {
                    from = 0;
                    self.tokens.push(std::mem::take(&mut inner_token));
                }
                from += tkn.delta_start as usize;
                inner_token.push(Token { from, len: tkn.length, token_type: tkn.token_type as usize });
            }
            self.tokens.push(inner_token);
        }
        self.tokens.len() > 1
    }

    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult) {
        let tokens = match tokens {
            SemanticTokensRangeResult::Partial(data) => data.data,
            SemanticTokensRangeResult::Tokens(data) => data.data,
        };
        let mut line_idx = 0;
        let mut from = 0;
        for token in tokens {
            if token.delta_line != 0 {
                from = 0;
                line_idx += token.delta_line as usize;
            }
            from += token.delta_start as usize;
            self.tokens[line_idx].push(Token { from, len: token.length, token_type: token.token_type as usize });
            self.ignores.clear();
        }
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
        if self.tokens.len() <= 1 {
            // ensures that there is fully mapped tokens before doing normal processing
            client.file_did_change(path, version, events.drain(..).map(|(_, edit)| edit).collect()).ok()?;
            return client.request_full_tokens(path).map(LSPResponseType::Tokens);
        }
        match events.len() {
            0 => None,
            1 => {
                let (meta, edit) = events.remove(0);
                meta.correct_tokens(&mut self.tokens, &mut self.ignores);
                client.file_did_change(path, version, vec![edit]).ok()?;
                client.request_partial_tokens(path, meta.build_range(content)).map(LSPResponseType::TokensPartial)
            }
            _ => {
                let edits = events
                    .drain(..)
                    .map(|(meta, edit)| {
                        meta.correct_tokens(&mut self.tokens, &mut self.ignores);
                        edit
                    })
                    .collect::<Vec<_>>();
                client.file_did_change(path, version, edits).ok()?;
                client.request_full_tokens(path).map(LSPResponseType::Tokens)
            }
        }
    }

    pub fn reset(&mut self) {
        self.brackets.reset();
    }

    pub fn build_line<'a>(&mut self, idx: usize, init: Vec<Span<'a>>, content: &'a str) -> Line<'a> {
        let line = if self.ignores.contains(&idx) || self.legend.is_empty() || self.tokens.len() <= 1 {
            let mut internal_build = SpansBuffer::new(init);
            internal_build.process(self, content, idx);
            internal_build.collect()
        } else {
            self.process_tokens(idx, content, init)
        };
        Line::from(line)
    }

    pub fn process_tokens<'a>(&mut self, line_idx: usize, content: &'a str, mut spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
        let mut style = Style { fg: Some(Color::White), ..Default::default() };
        let mut len: u32 = 0;
        let mut token_num = 0;
        let token_line = self.tokens.get(line_idx);
        let diagnostic = self.diagnostics.get(&line_idx);
        for (idx, ch) in content.char_indices() {
            len = len.saturating_sub(1);
            if len == 0 {
                if let Some(syntax_line) = token_line {
                    if let Some(t) = syntax_line.get(token_num) {
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
            }
            self.set_diagnostic_style(idx, &mut style, diagnostic);
            self.set_select(&mut style, &idx);
            self.brackets.map_style(ch, &mut style);
            spans.push(Span::styled(ch.to_string(), style));
            style.add_modifier = Modifier::empty();
            style.bg = None;
        }
        if let Some(diagnostic) = diagnostic {
            spans.extend(diagnostic.data.iter().map(|d| d.span.clone()));
        }
        spans
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

#[derive(Debug)]
pub struct Token {
    from: usize,
    len: u32,
    token_type: usize,
}
