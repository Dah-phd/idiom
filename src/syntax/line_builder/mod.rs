mod brackets;
use super::Lexer;
pub use brackets::BracketColors;
use lsp_types::Range;
use lsp_types::{Diagnostic, DiagnosticSeverity};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn build_line<'a>(lexer: &mut Lexer, idx: usize, content: &'a str) -> Line<'a> {
    let select_range = lexer.line_select(idx, content.len());
    let mut spans = vec![Span::styled(
        get_line_num(idx, lexer.max_digits),
        Style { fg: Some(Color::Gray), ..Default::default() },
    )];
    if select_range.is_some() && content.is_empty() {
        spans.push(Span::styled(" ", Style { bg: Some(lexer.theme.selected), ..Default::default() }));
        Line::from(spans)
    } else {
        let mut line_buf = SpansBuffer::new(spans, select_range);
        let mut buffer = Vec::new();
        if let Some(diagnostics) = &lexer.diagnostics {
            for diagnostic in diagnostics.diagnostics.iter() {
                if idx == diagnostic.range.start.line as usize {
                    match diagnostic.severity {
                        Some(severity) => match severity {
                            DiagnosticSeverity::ERROR => {
                                line_buf.eror.replace(add_span(&mut buffer, diagnostic, content.len(), Color::Red))
                            }
                            DiagnosticSeverity::WARNING => line_buf.warn.replace(add_span(
                                &mut buffer,
                                diagnostic,
                                content.len(),
                                Color::LightYellow,
                            )),
                            _ => line_buf.info.replace(add_span(&mut buffer, diagnostic, content.len(), Color::Gray)),
                        },
                        None => line_buf.info.replace(add_span(&mut buffer, diagnostic, content.len(), Color::Gray)),
                    };
                }
            }
        }
        line_buf.process(lexer, content);
        line_buf.append(&mut buffer);
        line_buf.collect()
    }
}

pub struct SpansBuffer<'a> {
    spans: Vec<Span<'a>>,
    eror: Option<std::ops::Range<usize>>,
    warn: Option<std::ops::Range<usize>>,
    info: Option<std::ops::Range<usize>>,
    select_range: Option<std::ops::Range<usize>>,
    token_buffer: String,
    last_reset: usize,
}

impl<'a> SpansBuffer<'a> {
    fn new(spans: Vec<Span<'a>>, select_range: Option<std::ops::Range<usize>>) -> Self {
        Self { spans, eror: None, warn: None, info: None, select_range, token_buffer: String::new(), last_reset: 0 }
    }

    fn push(&mut self, idx: usize, ch: char, color: Color) {
        self.spans.push(Span::styled(
            ch.to_string(),
            build_style(idx, &self.select_range, &self.eror, &self.warn, &self.info, color),
        ))
    }

    fn push_reset(&mut self, idx: usize, ch: char, color: Color) {
        self.push(idx, ch, color);
        self.token_buffer.clear();
        self.last_reset = idx + 1;
    }

    fn push_token(&mut self, idx: usize, ch: char, color: Color) {
        self.push(idx, ch, color);
        self.token_buffer.push(ch);
    }

    fn update_fg(&mut self, fg: Color) {
        // offset for line number
        for s in self.spans[1 + self.last_reset..].iter_mut() {
            s.style.fg.replace(fg);
        }
    }

    fn append(&mut self, other: &mut Vec<Span<'a>>) {
        self.spans.append(other)
    }

    fn collect(self) -> Line<'a> {
        Line::from(self.spans)
    }

    pub fn process(&mut self, lexer: &mut Lexer, content: &str) {
        let mut str_open = false;
        let mut chr_open = false;
        let mut is_class = false;
        for (idx, ch) in content.char_indices() {
            if str_open {
                self.push(idx, ch, lexer.theme.string);
                if ch == '"' {
                    str_open = false;
                    self.last_reset = idx + 1;
                }
                continue;
            }
            if chr_open {
                self.push(idx, ch, lexer.theme.string);
                if ch == '\'' {
                    chr_open = false;
                    self.last_reset = idx + 1;
                }
                continue;
            }
            if is_class {
                if ch.is_alphabetic() || ch == '_' || ch == '-' {
                    self.push(idx, ch, lexer.theme.class_or_struct);
                    continue;
                }
                is_class = false;
            }
            match ch {
                ' ' => {
                    if lexer.lang.key_words.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.key_words);
                    }
                    if lexer.lang.frow_control.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.flow_control);
                    }
                    self.push_reset(idx, ch, Color::White);
                }
                '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' | '|' => {
                    if lexer.lang.key_words.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.key_words);
                    }
                    if lexer.lang.frow_control.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.flow_control);
                    }
                    self.push_reset(idx, ch, Color::White);
                }
                ':' => {
                    if lexer.lang.key_words.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.key_words);
                    }
                    if lexer.lang.frow_control.contains(&self.token_buffer.as_str()) {
                        self.update_fg(lexer.theme.flow_control)
                    }
                    self.push_reset(idx, ch, Color::White);
                }
                '"' => {
                    str_open = true;
                    self.push_reset(idx, ch, lexer.theme.string);
                }
                '\'' => {
                    chr_open = true;
                    self.push_reset(idx, ch, lexer.theme.string);
                }
                '!' => {
                    self.update_fg(lexer.theme.key_words);
                    let color = if self.token_buffer.is_empty() { Color::White } else { lexer.theme.key_words };
                    self.push_reset(idx, ch, color);
                }
                '(' => {
                    if let Some(first) = self.token_buffer.chars().next() {
                        let tc = if first.is_uppercase() { lexer.theme.key_words } else { lexer.theme.functions };
                        self.update_fg(tc);
                    }
                    self.push(idx, ch, lexer.brackets.open());
                    self.last_reset = idx + 1;
                }
                ')' => self.push_reset(idx, ch, lexer.brackets.close()),
                '{' => self.push_reset(idx, ch, lexer.brackets.curly_open()),
                '}' => self.push_reset(idx, ch, lexer.brackets.curly_close()),
                '[' => self.push_reset(idx, ch, lexer.brackets.square_open()),
                ']' => self.push_reset(idx, ch, lexer.brackets.square_close()),
                _ => {
                    if ch.is_numeric() {
                        self.push(idx, ch, lexer.theme.numeric);
                        self.last_reset = idx + 1;
                    } else if ch.is_uppercase() && self.token_buffer.is_empty() {
                        self.push(idx, ch, lexer.theme.class_or_struct);
                        is_class = true;
                    } else {
                        self.push_token(idx, ch, lexer.theme.default);
                    }
                }
            }
        }
    }
}

fn add_span(buffer: &mut Vec<Span<'_>>, diagnostic: &Diagnostic, max: usize, c: Color) -> std::ops::Range<usize> {
    buffer.push(Span::styled(format!("    {}", diagnostic.message), Style { fg: Some(c), ..Default::default() }));
    process_range(diagnostic.range, max)
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = (idx + 1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}

fn process_range(r: Range, max: usize) -> std::ops::Range<usize> {
    if r.start.line == r.end.line {
        return r.start.character as usize..r.end.character as usize;
    }
    r.start.character as usize..max
}

fn get_sel_style(r: &Option<std::ops::Range<usize>>, idx: usize) -> Option<Color> {
    if let Some(range) = r {
        if range.contains(&idx) {
            return Some(Color::Rgb(72, 72, 72));
        }
    }
    None
}

fn build_style(
    idx: usize,
    sel: &Option<std::ops::Range<usize>>,
    eror: &Option<std::ops::Range<usize>>,
    warn: &Option<std::ops::Range<usize>>,
    info: &Option<std::ops::Range<usize>>,
    col: Color,
) -> Style {
    let style = Style { fg: Some(col), bg: get_sel_style(sel, idx), ..Default::default() };
    if let Some(range) = eror {
        if range.contains(&idx) {
            return style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Red);
        }
    }
    if let Some(range) = warn {
        if range.contains(&idx) {
            return style.add_modifier(Modifier::UNDERLINED).underline_color(Color::LightYellow);
        }
    }
    if let Some(range) = info {
        if range.contains(&idx) {
            return style.add_modifier(Modifier::UNDERLINED).underline_color(Color::Gray);
        }
    }
    style
}
