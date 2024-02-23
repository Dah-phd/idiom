use super::{context::LineBuilderContext, diagnostics::DiagnosticLine, LineBuilder};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

/// used to map markup lines based on info
/// functionality as line number, wrap, cursor position, select are handled by the logic in LineBuilder
#[allow(dead_code)]
pub fn mark_down_line<'a>(
    // TODO enable
    _builder: &mut LineBuilder,
    _idx: usize,
    content: &str,
    mut buffer: Vec<Span<'a>>,
) -> Vec<Span<'a>> {
    for ch in content.chars() {
        match ch {
            '#' => buffer.push(Span::styled(ch.to_string(), Style { fg: Some(Color::Blue), ..Default::default() })),
            '*' => buffer.push(Span::styled(ch.to_string(), Style { fg: Some(Color::Blue), ..Default::default() })),
            '[' | ']' => {
                buffer.push(Span::styled(ch.to_string(), Style { fg: Some(Color::Magenta), ..Default::default() }))
            }
            _ => buffer.push(Span::raw(ch.to_string())),
        }
    }
    buffer
}

/// build generic styled line based on info on lang and theme (calls LineBuilder functionality)
/// functionality as line number, wrap, cursor position, select are handled by the logic in LineBuilder
pub fn generic_line(
    builder: &LineBuilder,
    idx: usize,
    content: &str,
    ctx: &mut LineBuilderContext,
    mut buffer: Vec<Span<'static>>,
) -> Vec<Span<'static>> {
    if builder.lang.is_comment(content) {
        buffer.extend(content.char_indices().map(|(idx, ch)| {
            let mut style = Style { fg: Some(builder.theme.comment), ..Default::default() };
            ctx.set_select(&mut style, &idx, builder.theme.selected);
            Span::styled(ch.to_string(), style)
        }));
        return ctx.format_with_info(idx, None, buffer);
    }
    let mut buf = SpanBuffer::new(buffer, builder.theme.selected);
    let mut chars = content.char_indices().peekable();
    let diagnostic = builder.diagnostics.get(&idx);
    while let Some((idx, ch)) = chars.next() {
        if buf.handled_edgecases(idx, ch, diagnostic, builder, ctx) {
            continue;
        }
        match ch {
            ' ' => {
                if builder.lang.frow_control.contains(&buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.flow_control);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, ctx);
            }
            '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' | '|' => {
                if builder.lang.frow_control.contains(&buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.flow_control);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, ctx);
            }
            ':' => {
                if matches!(chars.peek(), Some((.., next_ch)) if &':' == next_ch) {
                    buf.update_fg(builder.theme.class_or_struct);
                } else if builder.lang.is_keyword(buf.token_buffer.as_str()) {
                    buf.update_fg(builder.theme.key_words);
                }
                buf.push_reset(idx, ch, Color::White, diagnostic, ctx);
            }
            '"' => {
                buf.str_open = true;
                buf.push_reset(idx, ch, builder.theme.string, diagnostic, ctx);
            }
            '\'' => buf.handle_lifetime_apostrophe(idx, ch, builder, ctx, diagnostic),
            '!' => {
                buf.update_fg(builder.theme.key_words);
                let color = if buf.token_buffer.is_empty() { Color::White } else { builder.theme.key_words };
                buf.push_reset(idx, ch, color, diagnostic, ctx);
            }
            '(' => {
                if let Some(first) = buf.token_buffer.chars().next() {
                    let tc = if first.is_uppercase() { builder.theme.key_words } else { builder.theme.functions };
                    buf.update_fg(tc);
                }
                buf.push(idx, ch, ctx.brackets.open_round(), diagnostic, ctx);
                buf.last_reset = idx + 1;
            }
            ')' => buf.push_reset(idx, ch, ctx.brackets.close_round(), diagnostic, ctx),
            '{' => buf.push_reset(idx, ch, ctx.brackets.open_curly(), diagnostic, ctx),
            '}' => buf.push_reset(idx, ch, ctx.brackets.close_curly(), diagnostic, ctx),
            '[' => buf.push_reset(idx, ch, ctx.brackets.open_square(), diagnostic, ctx),
            ']' => buf.push_reset(idx, ch, ctx.brackets.close_square(), diagnostic, ctx),
            _ => {
                if ch.is_numeric() {
                    buf.push(idx, ch, builder.theme.numeric, diagnostic, ctx);
                    buf.last_reset = idx + 1;
                } else if ch.is_uppercase() && buf.token_buffer.is_empty() {
                    buf.push(idx, ch, builder.theme.class_or_struct, diagnostic, ctx);
                    buf.is_class = true;
                } else {
                    buf.push_token(idx, ch, builder.theme.default, diagnostic, ctx);
                }
            }
        }
    }
    ctx.format_with_info(idx, diagnostic, buf.buffer)
}

#[derive(Default)]
struct SpanBuffer<'a> {
    token_buffer: String,
    last_reset: usize,
    last_char: char,
    str_open: bool,
    chr_open: bool,
    is_class: bool,
    is_keyword: bool,
    buffer: Vec<Span<'a>>,
    select_color: Color,
}

impl<'a> SpanBuffer<'a> {
    fn new(buffer: Vec<Span<'static>>, select_color: Color) -> Self {
        Self { last_reset: buffer.len(), buffer, last_char: '\n', select_color, ..Default::default() }
    }

    fn push(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLine>,
        ctx: &LineBuilderContext,
    ) {
        self.buffer.push(Span::styled(ch.to_string(), self.build_style(idx, color, diagnostic, ctx)));
        self.last_char = ch;
    }

    fn push_reset(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLine>,
        ctx: &LineBuilderContext,
    ) {
        self.push(idx, ch, color, diagnostic, ctx);
        self.token_buffer.clear();
        self.last_reset = idx + 1;
    }

    fn push_token(
        &mut self,
        idx: usize,
        ch: char,
        color: Color,
        diagnostic: Option<&DiagnosticLine>,
        ctx: &LineBuilderContext,
    ) {
        self.push(idx, ch, color, diagnostic, ctx);
        self.token_buffer.push(ch);
    }

    fn handled_edgecases(
        &mut self,
        idx: usize,
        ch: char,
        diagnostic: Option<&DiagnosticLine>,
        builder: &LineBuilder,
        ctx: &LineBuilderContext,
    ) -> bool {
        if self.str_open {
            self.push(idx, ch, builder.theme.string, diagnostic, ctx);
            if ch == '"' {
                self.str_open = false;
                self.last_reset = idx + 1;
            }
            return true;
        }
        if self.chr_open {
            self.push(idx, ch, builder.theme.string, diagnostic, ctx);
            if ch == '\'' {
                self.chr_open = false;
                self.last_reset = idx + 1;
            }
            return true;
        }
        if self.is_class {
            if ch.is_alphabetic() || ch == '_' || ch == '-' {
                self.push(idx, ch, builder.theme.class_or_struct, diagnostic, ctx);
                return true;
            }
            self.is_class = false;
        }
        if self.is_keyword {
            if ch.is_alphabetic() || ch == '_' {
                self.push(idx, ch, builder.theme.key_words, diagnostic, ctx);
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
        ctx: &LineBuilderContext,
        diagnostic: Option<&DiagnosticLine>,
    ) {
        if self.last_char != '<' && self.last_char != '&' {
            self.chr_open = true;
            self.push_reset(idx, ch, builder.theme.string, diagnostic, ctx);
        } else {
            self.is_keyword = true;
            self.push_reset(idx, ch, builder.theme.key_words, diagnostic, ctx);
        };
    }

    fn update_fg(&mut self, fg: Color) {
        for s in self.buffer.iter_mut().skip(self.last_reset) {
            s.style.fg.replace(fg);
        }
    }

    fn build_style(
        &self,
        idx: usize,
        color: Color,
        diagnostic: Option<&DiagnosticLine>,
        ctx: &LineBuilderContext,
    ) -> Style {
        let mut style = Style { fg: Some(color), ..Default::default() };
        if let Some(diagnostic) = diagnostic {
            diagnostic.set_diagnostic_style(idx, &mut style);
        }
        if matches!(&ctx.select_range, Some(range) if range.contains(&idx)) {
            style.bg.replace(self.select_color);
        }
        style
    }
}
