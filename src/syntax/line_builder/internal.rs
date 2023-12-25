use super::diagnostics::DiagnosticLines;
use super::LineBuilder;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub struct SpansBuffer<'a> {
    offset: usize,
    spans: Vec<Span<'a>>,
    token_buffer: String,
    last_reset: usize,
    last_char: char,
    str_open: bool,
    chr_open: bool,
    is_class: bool,
    is_keyword: bool,
}

impl<'a> SpansBuffer<'a> {
    pub fn new(spans: Vec<Span<'a>>) -> Self {
        Self {
            offset: spans.len(),
            spans,
            token_buffer: String::new(),
            last_reset: 0,
            last_char: '\n',
            str_open: false,
            chr_open: false,
            is_class: false,
            is_keyword: false,
        }
    }

    fn build_style(
        &self,
        idx: usize,
        color: Color,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
    ) -> Style {
        let mut style = Style { fg: Some(color), ..Default::default() };
        builder.set_diagnostic_style(idx, &mut style, diagnostic);
        builder.set_select(&mut style, &idx);
        style
    }

    fn push(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
    ) {
        self.spans.push(Span::styled(ch.to_string(), self.build_style(idx, color, diagnostic, builder)));
        self.last_char = ch;
    }

    fn push_reset(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
    ) {
        self.push(idx, ch, color, diagnostic, builder);
        self.token_buffer.clear();
        self.last_reset = idx + 1;
    }

    fn push_token(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
    ) {
        self.push(idx, ch, color, diagnostic, builder);
        self.token_buffer.push(ch);
    }

    fn update_fg(&mut self, fg: Color) {
        // offset for line number
        for s in self.spans[self.offset + self.last_reset..].iter_mut() {
            s.style.fg.replace(fg);
        }
    }

    pub fn collect(self) -> Vec<Span<'a>> {
        self.spans
    }

    fn handle_lifetime_apostrophe(
        &mut self,
        idx: usize,
        ch: char,
        builder: &LineBuilder,
        diagnostic: Option<&DiagnosticLines>,
    ) {
        if self.last_char != '<' && self.last_char != '&' {
            self.chr_open = true;
            self.push_reset(idx, ch, builder.theme.string, diagnostic, builder);
        } else {
            self.is_keyword = true;
            self.push_reset(idx, ch, builder.theme.key_words, diagnostic, builder);
        };
    }

    fn handled_edgecases(
        &mut self,
        idx: usize,
        ch: char,
        builder: &LineBuilder,
        diagnostic: Option<&DiagnosticLines>,
    ) -> bool {
        if self.str_open {
            self.push(idx, ch, builder.theme.string, diagnostic, builder);
            if ch == '"' {
                self.str_open = false;
                self.last_reset = idx + 1;
            }
            return true;
        }
        if self.chr_open {
            self.push(idx, ch, builder.theme.string, diagnostic, builder);
            if ch == '\'' {
                self.chr_open = false;
                self.last_reset = idx + 1;
            }
            return true;
        }
        if self.is_class {
            if ch.is_alphabetic() || ch == '_' || ch == '-' {
                self.push(idx, ch, builder.theme.class_or_struct, diagnostic, builder);
                return true;
            }
            self.is_class = false;
        }
        if self.is_keyword {
            if ch.is_alphabetic() || ch == '_' {
                self.push(idx, ch, builder.theme.key_words, diagnostic, builder);
                return true;
            }
            self.is_keyword = false;
        }
        false
    }

    pub fn process(&mut self, builder: &mut LineBuilder, content: &str, idx: usize) {
        let mut chars = content.char_indices().peekable();
        let diagnostic = builder.diagnostics.get(&idx);
        while let Some((idx, ch)) = chars.next() {
            if self.handled_edgecases(idx, ch, builder, diagnostic) {
                continue;
            }
            match ch {
                ' ' => {
                    if builder.lang.frow_control.contains(&self.token_buffer.as_str()) {
                        self.update_fg(builder.theme.flow_control);
                    } else if builder.lang.is_keyword(self.token_buffer.as_str()) {
                        self.update_fg(builder.theme.key_words);
                    }
                    self.push_reset(idx, ch, Color::White, diagnostic, builder);
                }
                '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' | '|' => {
                    if builder.lang.frow_control.contains(&self.token_buffer.as_str()) {
                        self.update_fg(builder.theme.flow_control);
                    } else if builder.lang.is_keyword(self.token_buffer.as_str()) {
                        self.update_fg(builder.theme.key_words);
                    }
                    self.push_reset(idx, ch, Color::White, diagnostic, builder);
                }
                ':' => {
                    if matches!(chars.peek(), Some((.., next_ch)) if &':' == next_ch) {
                        self.update_fg(builder.theme.class_or_struct);
                    } else if builder.lang.is_keyword(self.token_buffer.as_str()) {
                        self.update_fg(builder.theme.key_words);
                    }
                    self.push_reset(idx, ch, Color::White, diagnostic, builder);
                }
                '"' => {
                    self.str_open = true;
                    self.push_reset(idx, ch, builder.theme.string, diagnostic, builder);
                }
                '\'' => self.handle_lifetime_apostrophe(idx, ch, builder, diagnostic),
                '!' => {
                    self.update_fg(builder.theme.key_words);
                    let color = if self.token_buffer.is_empty() { Color::White } else { builder.theme.key_words };
                    self.push_reset(idx, ch, color, diagnostic, builder);
                }
                '(' => {
                    if let Some(first) = self.token_buffer.chars().next() {
                        let tc = if first.is_uppercase() { builder.theme.key_words } else { builder.theme.functions };
                        self.update_fg(tc);
                    }
                    self.push(idx, ch, builder.brackets.open(), diagnostic, builder);
                    self.last_reset = idx + 1;
                }
                ')' => self.push_reset(idx, ch, builder.brackets.close(), diagnostic, builder),
                '{' => self.push_reset(idx, ch, builder.brackets.curly_open(), diagnostic, builder),
                '}' => self.push_reset(idx, ch, builder.brackets.curly_close(), diagnostic, builder),
                '[' => self.push_reset(idx, ch, builder.brackets.square_open(), diagnostic, builder),
                ']' => self.push_reset(idx, ch, builder.brackets.square_close(), diagnostic, builder),
                _ => {
                    if ch.is_numeric() {
                        self.push(idx, ch, builder.theme.numeric, diagnostic, builder);
                        self.last_reset = idx + 1;
                    } else if ch.is_uppercase() && self.token_buffer.is_empty() {
                        self.push(idx, ch, builder.theme.class_or_struct, diagnostic, builder);
                        self.is_class = true;
                    } else {
                        self.push_token(idx, ch, builder.theme.default, diagnostic, builder);
                    }
                }
            }
        }
        if let Some(diagnostic) = diagnostic {
            self.spans.extend(diagnostic.data.iter().map(|d| d.span.clone()));
        }
    }
}
