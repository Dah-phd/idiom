use super::diagnostics::DiagnosticLines;
use super::LineBuilder;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub fn generic_line<'a>(builder: &mut LineBuilder, idx: usize, content: &str, buffer: Vec<Span<'a>>) -> Vec<Span<'a>> {
    let mut buf = SpanBuffer::from(buffer);
    let mut chars = content.char_indices().peekable();
    let diagnostic = builder.diagnostics.get(&idx);
    while let Some((idx, ch)) = chars.next() {
        if buf.handled_edgecases(idx, ch, diagnostic, builder) {
            continue;
        }
        match ch {
            ' ' => {
                if builder.lang.frow_control.contains(&buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.flow_control);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, builder);
            }
            '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' | '|' => {
                if builder.lang.frow_control.contains(&buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.flow_control);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, builder);
            }
            ':' => {
                if matches!(chars.peek(), Some((.., next_ch)) if &':' == next_ch) {
                    buf.update_fg(builder.theme.class_or_struct);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, builder);
            }
            '"' => {
                buf.str_open = true;
                buf.push_reset(idx, ch, builder.theme.string, diagnostic, builder);
            }
            '\'' => buf.handle_lifetime_apostrophe(idx, ch, builder, diagnostic),
            '!' => {
                buf.update_fg(builder.theme.key_words);
                let color = if buf.token_buffer.is_empty() { Color::White } else { builder.theme.key_words };
                buf.push_reset(idx, ch, color, diagnostic, builder);
            }
            '(' => {
                if let Some(first) = buf.token_buffer.chars().next() {
                    let tc = if first.is_uppercase() { builder.theme.key_words } else { builder.theme.functions };
                    buf.update_fg(tc);
                }
                buf.push(idx, ch, builder.brackets.open(), diagnostic, builder);
                buf.last_reset = idx + 1;
            }
            ')' => buf.push_reset(idx, ch, builder.brackets.close(), diagnostic, builder),
            '{' => buf.push_reset(idx, ch, builder.brackets.curly_open(), diagnostic, builder),
            '}' => buf.push_reset(idx, ch, builder.brackets.curly_close(), diagnostic, builder),
            '[' => buf.push_reset(idx, ch, builder.brackets.square_open(), diagnostic, builder),
            ']' => buf.push_reset(idx, ch, builder.brackets.square_close(), diagnostic, builder),
            _ => {
                if ch.is_numeric() {
                    buf.push(idx, ch, builder.theme.numeric, diagnostic, builder);
                    buf.last_reset = idx + 1;
                } else if ch.is_uppercase() && buf.token_buffer.is_empty() {
                    buf.push(idx, ch, builder.theme.class_or_struct, diagnostic, builder);
                    buf.is_class = true;
                } else {
                    buf.push_token(idx, ch, builder.theme.default, diagnostic, builder);
                }
            }
        }
    }
    if let Some(diagnostic) = diagnostic {
        buf.buffer.extend(diagnostic.data.iter().map(|d| d.span.clone()));
    }
    buf.buffer
}

#[derive(Default)]
struct SpanBuffer<'a> {
    token_buffer: String,
    offset: usize,
    last_reset: usize,
    last_char: char,
    str_open: bool,
    chr_open: bool,
    is_class: bool,
    is_keyword: bool,
    buffer: Vec<Span<'a>>,
}

impl<'a> SpanBuffer<'a> {
    fn push(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
    ) {
        self.buffer.push(Span::styled(ch.to_string(), SpanBuffer::build_style(idx, color, diagnostic, builder)));
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

    fn handled_edgecases(
        &mut self,
        idx: usize,
        ch: char,
        diagnostic: Option<&DiagnosticLines>,
        builder: &LineBuilder,
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

    fn update_fg(&mut self, fg: Color) {
        for s in self.buffer[self.offset + self.last_reset..].iter_mut() {
            s.style.fg.replace(fg);
        }
    }

    fn build_style(idx: usize, color: Color, diagnostic: Option<&DiagnosticLines>, builder: &LineBuilder) -> Style {
        let mut style = Style { fg: Some(color), ..Default::default() };
        builder.set_diagnostic_style(idx, &mut style, diagnostic);
        builder.set_select(&mut style, &idx);
        style
    }
}

impl<'a> From<Vec<Span<'a>>> for SpanBuffer<'a> {
    fn from(buffer: Vec<Span<'a>>) -> Self {
        Self { offset: buffer.len(), buffer, last_char: '\n', ..Default::default() }
    }
}
