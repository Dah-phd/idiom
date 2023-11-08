mod brackets;
mod internal;
mod legend;
use brackets::BracketColors;
use internal::SpansBuffer;
use legend::{ColorResult, Legend};

use lsp_types::{
    Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams, SemanticTokensResult, SemanticTokensServerCapabilities,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::ops::Range;

use crate::{configs::FileType, syntax::langs::Lang, syntax::Theme};

#[derive(Debug, Default)]
pub struct LineBuilder {
    pub tokens: Vec<Vec<Token>>,
    pub legend: Legend,
    pub theme: Theme,
    pub lang: Lang,
    brackets: BracketColors,
    eror: Vec<Range<usize>>,
    warn: Vec<Range<usize>>,
    info: Vec<Range<usize>>,
    pub select_range: Option<Range<usize>>,
    pub waiting: bool,
    pub text_is_updated: bool,
}

impl From<(Theme, Lang)> for LineBuilder {
    fn from(value: (Theme, Lang)) -> Self {
        Self {
            tokens: Vec::new(),
            legend: Legend::default(),
            theme: value.0,
            lang: value.1,
            brackets: BracketColors::default(),
            eror: Vec::new(),
            warn: Vec::new(),
            info: Vec::new(),
            select_range: None,
            waiting: false,
            text_is_updated: false,
        }
    }
}

impl LineBuilder {
    pub fn set_tokens(&mut self, tokens_res: SemanticTokensResult) {
        let mut tokens = Vec::new();
        let mut inner_token = Vec::new();
        let mut from = 0;
        if let SemanticTokensResult::Tokens(tkns) = tokens_res {
            for tkn in tkns.data {
                for _ in 0..tkn.delta_line {
                    from = 0;
                    tokens.push(std::mem::take(&mut inner_token));
                }
                from += tkn.delta_start as usize;
                inner_token.push(Token { from, len: tkn.length, token_type: tkn.token_type as usize });
            }
            tokens.push(inner_token);
        }
        self.tokens = tokens;
        self.waiting = false;
        self.text_is_updated = false;
    }

    pub fn reset(&mut self) {
        self.brackets.reset();
    }

    pub fn build_line<'a>(
        &mut self,
        idx: usize,
        init: Vec<Span<'a>>,
        diagnostics: &Option<PublishDiagnosticsParams>,
        content: &'a str,
    ) -> Line<'a> {
        let mut buffer = Vec::new();
        self.eror.clear();
        self.warn.clear();
        self.info.clear();
        if let Some(diagnostics) = &diagnostics {
            for diagnostic in diagnostics.diagnostics.iter() {
                if idx == diagnostic.range.start.line as usize {
                    match diagnostic.severity {
                        Some(severity) => match severity {
                            DiagnosticSeverity::ERROR => {
                                self.eror.push(add_span(&mut buffer, diagnostic, content.len(), Color::Red))
                            }
                            DiagnosticSeverity::WARNING => {
                                self.warn.push(add_span(&mut buffer, diagnostic, content.len(), Color::LightYellow))
                            }
                            _ => self.info.push(add_span(&mut buffer, diagnostic, content.len(), Color::Gray)),
                        },
                        None => self.info.push(add_span(&mut buffer, diagnostic, content.len(), Color::Gray)),
                    };
                }
            }
        }
        let mut line = if self.tokens.is_empty() || self.legend.is_empty() || self.waiting || self.text_is_updated {
            let mut internal_build = SpansBuffer::new(init, self.select_range.take());
            internal_build.process(self, content);
            internal_build.collect()
        } else {
            self.process_tokens(idx, content, init)
        };
        line.append(&mut buffer);
        Line::from(line)
    }

    pub fn process_tokens<'a>(&mut self, line_idx: usize, content: &'a str, mut spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
        let mut style = Style { fg: Some(Color::White), ..Default::default() };
        let mut len: u32 = 0;
        let mut token_num = 0;
        let token_line = self.tokens.get(line_idx);
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
            self.set_diagnostic_style(idx, &mut style);
            if matches!(&self.select_range, Some(range) if range.contains(&idx)) {
                style.bg.replace(self.theme.selected);
            } else {
                style.bg = None;
            }
            self.brackets.map_style(ch, &mut style);
            spans.push(Span::styled(ch.to_string(), style));
        }
        spans
    }

    fn set_diagnostic_style(&self, idx: usize, style: &mut Style) {
        for range in &self.eror {
            if range.contains(&idx) {
                *style = style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Red);
                return;
            }
        }
        for range in &self.warn {
            if range.contains(&idx) {
                *style = style.add_modifier(Modifier::UNDERLINED).underline_color(Color::LightYellow);
                return;
            }
        }
        for range in &self.info {
            if range.contains(&idx) {
                *style = style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Gray);
                return;
            }
        }
        style.add_modifier = Modifier::empty();
    }

    fn handle_keywords(&self, word: &str) -> Color {
        if self.lang.frow_control.contains(&word) {
            return self.theme.flow_control;
        }
        self.theme.key_words
    }

    pub fn map_styles(&mut self, ft: &FileType, tokens_res: &Option<SemanticTokensServerCapabilities>) {
        if let Some(capabilities) = tokens_res {
            self.legend.map_styles(ft, &self.theme, capabilities)
        }
    }

    pub fn should_update(&self) -> bool {
        !self.waiting && (self.tokens.len() < 2 || self.text_is_updated)
    }

    pub fn new_line(&mut self, index: usize) {
        self.tokens.insert(index, Vec::new());
    }
}

#[derive(Debug)]
pub struct Token {
    from: usize,
    len: u32,
    token_type: usize,
}

fn add_span(buffer: &mut Vec<Span<'_>>, diagnostic: &Diagnostic, max: usize, c: Color) -> std::ops::Range<usize> {
    buffer.push(Span::styled(format!("    {}", diagnostic.message), Style { fg: Some(c), ..Default::default() }));
    process_range(diagnostic.range, max)
}

fn process_range(r: lsp_types::Range, max: usize) -> std::ops::Range<usize> {
    if r.start.line == r.end.line {
        return r.start.character as usize..r.end.character as usize;
    }
    r.start.character as usize..max
}
