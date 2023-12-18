mod brackets;
mod internal;
mod langs;
mod legend;
use brackets::BracketColors;
use internal::SpansBuffer;
use langs::Lang;
use legend::{ColorResult, Legend};

use lsp_types::{
    Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams, SemanticTokensRangeResult, SemanticTokensResult,
    SemanticTokensServerCapabilities, TextDocumentContentChangeEvent,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Range,
    path::Path,
};

use crate::{global_state::GlobalState, lsp::LSPClient, syntax::Theme, workspace::actions::EditMetaData};

use super::modal::LSPResponseType;

#[derive(Debug)]
pub struct LineBuilder {
    pub tokens: Vec<Vec<Token>>,
    pub legend: Legend,
    pub theme: Theme,
    pub lang: Lang,
    brackets: BracketColors,
    diagnostics: HashMap<usize, DiagnosticData>,
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
            brackets: BracketColors::default(),
            diagnostics: HashMap::new(),
            select_range: None,
            file_was_saved: true,
            ignores: Vec::new(),
        }
    }

    pub fn set_diganostics(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.clear();
        if self.file_was_saved {
            self.diagnostics_full(params);
        } else {
            self.diagnostics_error(params);
        }
    }

    pub fn diagnostics_error(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.clear();
        for d in params.diagnostics {
            if !matches!(d.severity, Some(DiagnosticSeverity::ERROR)) {
                continue;
            }
            match self.diagnostics.entry(d.range.start.line as usize) {
                Entry::Occupied(mut e) => {
                    e.get_mut().append(d);
                }
                Entry::Vacant(e) => {
                    e.insert(d.into());
                }
            }
        }
    }

    pub fn diagnostics_full(&mut self, params: PublishDiagnosticsParams) {
        self.diagnostics.clear();
        for diagnostic in params.diagnostics {
            match self.diagnostics.entry(diagnostic.range.start.line as usize) {
                Entry::Occupied(mut e) => {
                    e.get_mut().append(diagnostic);
                }
                Entry::Vacant(e) => {
                    e.insert(diagnostic.into());
                }
            };
        }
        self.file_was_saved = false;
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

    pub fn set_tokens_partial(&mut self, tokens: SemanticTokensRangeResult, gs: &mut GlobalState) {
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
            gs.error(format!("{:?}", self.ignores));
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
        if self.tokens.len() <= 1 {
            // ensures that there is fully mapped tokens before doing normal processing
            client.file_did_change(path, version, events.drain(..).map(|(_, edit)| edit).collect()).ok()?;
            return client.full_tokens(path).map(LSPResponseType::Tokens);
        }
        match events.len() {
            0 => None,
            1 => {
                let (meta, edit) = events.remove(0);
                meta.correct_tokens(&mut self.tokens, &mut self.ignores);
                client.file_did_change(path, version, vec![edit]).ok()?;
                client.partial_tokens(path, meta.build_range(content)).map(LSPResponseType::TokensPartial)
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
                client.full_tokens(path).map(LSPResponseType::Tokens)
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
            spans.extend(diagnostic.spans.iter().cloned());
        }
        spans
    }

    fn set_diagnostic_style(&self, idx: usize, style: &mut Style, diagnostic: Option<&DiagnosticData>) {
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
struct DiagnosticData {
    spans: Vec<Span<'static>>,
    range_data: Vec<(usize, Option<usize>)>,
}

impl DiagnosticData {
    fn check_ranges(&self, idx: &usize) -> Option<Color> {
        for (range_idx, (start_idx, end_idx)) in self.range_data.iter().enumerate() {
            match end_idx {
                Some(end_idx) if (start_idx..end_idx).contains(&idx) => return self.spans[range_idx].style.fg,
                None if idx >= start_idx => return self.spans[range_idx].style.fg,
                _ => {}
            }
        }
        None
    }

    fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.range_data.insert(0, range_to_tuple(d.range));
                self.spans.insert(0, span_diagnostic(&d, Color::Red));
            }
            Some(DiagnosticSeverity::WARNING) => match self.spans[0].style.fg {
                Some(Color::Gray) => {
                    self.range_data.insert(0, range_to_tuple(d.range));
                    self.spans.insert(0, span_diagnostic(&d, Color::LightYellow));
                }
                _ => {
                    self.range_data.push(range_to_tuple(d.range));
                    self.spans.push(span_diagnostic(&d, Color::LightYellow));
                }
            },
            _ => {
                self.range_data.push(range_to_tuple(d.range));
                self.spans.push(span_diagnostic(&d, Color::Gray));
            }
        }
    }
}

impl From<Diagnostic> for DiagnosticData {
    fn from(diagnostic: Diagnostic) -> Self {
        let mut spans = Vec::new();
        match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => spans.push(span_diagnostic(&diagnostic, Color::Red)),
            Some(DiagnosticSeverity::WARNING) => spans.push(span_diagnostic(&diagnostic, Color::LightYellow)),
            _ => spans.push(span_diagnostic(&diagnostic, Color::Gray)),
        };
        Self { spans, range_data: vec![range_to_tuple(diagnostic.range)] }
    }
}

#[derive(Debug)]
pub struct Token {
    from: usize,
    len: u32,
    token_type: usize,
}

fn span_diagnostic(d: &Diagnostic, c: Color) -> Span<'static> {
    Span::styled(format!("    {}", d.message), Style { fg: Some(c), ..Default::default() })
}

fn range_to_tuple(r: lsp_types::Range) -> (usize, Option<usize>) {
    (r.start.character as usize, if r.start.line == r.end.line { Some(r.end.character as usize) } else { None })
}
