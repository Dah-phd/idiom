mod theme;
pub use self::theme::{Theme, DEFAULT_THEME_FILE};
use crate::components::editor::CursorPosition;
use crate::messages::FileType;
use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};
pub const COLORS: [Color; 3] = [Color::LightMagenta, Color::Yellow, Color::Blue];

#[derive(Debug)]
pub struct Lexer {
    pub select: Option<(CursorPosition, CursorPosition)>,
    pub theme: Theme,
    token_start: usize,
    select_at_line: Option<(usize, usize)>,
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    last_token: String,
    key_words: Vec<&'static str>,
    last_key_words: Vec<String>,
    max_digits: usize,
}

impl Default for Lexer {
    fn default() -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut", "crate"],
            select_at_line: None,
            curly: vec![],
            brackets: vec![],
            square: vec![],
            last_token: String::default(),
            token_start: 0,
            last_key_words: vec![],
            theme: Theme::default(),
            select: None,
            max_digits: 0,
        }
    }
}

impl Lexer {
    pub fn new(theme: Theme) -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut", "crate"],
            select_at_line: None,
            curly: vec![],
            brackets: vec![],
            square: vec![],
            last_token: String::default(),
            token_start: 0,
            last_key_words: vec![],
            theme,
            select: None,
            max_digits: 0,
        }
    }

    pub fn from_type(file_type: &FileType, theme: Theme) -> Self {
        #[allow(clippy::match_single_binding)]
        match file_type {
            _ => Self::new(theme),
        }
    }

    pub fn line_number_max_digits(&mut self, content: &[String]) -> usize {
        self.max_digits = if content.is_empty() {
            0
        } else {
            (content.len().ilog10() + 1) as usize
        };
        self.max_digits
    }

    pub fn reset(&mut self, select: Option<(&CursorPosition, &CursorPosition)>) {
        self.curly.clear();
        self.brackets.clear();
        self.square.clear();
        self.last_key_words.clear();
        self.select = select.map(|(from, to)| (*from, *to))
    }

    fn set_select_char_range(&mut self, at_line: usize, max_len: usize) {
        if let Some((from, to)) = self.select {
            if from.line > at_line || at_line > to.line {
                self.select_at_line = None;
            } else if from.line < at_line && at_line < to.line {
                self.select_at_line = Some((0, max_len));
            } else if from.line == at_line && at_line == to.line {
                self.select_at_line = Some((from.char, to.char));
            } else if from.line == at_line {
                self.select_at_line = Some((from.char, max_len));
            } else if to.line == at_line {
                self.select_at_line = Some((0, to.char))
            }
        } else {
            self.select_at_line = None
        }
    }

    fn process_line(&mut self, content: &str, spans: &mut Vec<Span>) {
        if content.starts_with("mod") | content.starts_with("use") {
            for (idx, ch) in content.chars().enumerate() {
                match ch {
                    ' ' => {
                        self.drain_buf_object(idx, spans);
                        self.last_token.push(ch);
                    }
                    '.' | '<' | '>' | ':' | '?' | '&' | '=' | '+' | '-' | ',' | ';' => {
                        self.drain_buf_object(idx, spans);
                        self.white_char(idx, ch, spans);
                    }
                    _ => self.last_token.push(ch),
                }
            }
            return;
        }
        let mut char_steam = content.chars().enumerate().peekable();
        while let Some((token_end, ch)) = char_steam.next() {
            match ch {
                ' ' => {
                    self.drain_buf(token_end, spans);
                    self.last_token.push(ch);
                }
                '.' | '<' | '>' | '?' | '&' | '=' | '+' | '-' | ',' | ';' => {
                    self.drain_buf(token_end, spans);
                    self.white_char(token_end, ch, spans);
                }
                ':' => {
                    if matches!(char_steam.peek(), Some((_, next_ch)) if next_ch == &':') {
                        self.drain_buf_colored(token_end, self.theme.class_or_struct, spans);
                        self.white_char(token_end, ch, spans);
                    } else {
                        self.drain_buf(token_end, spans);
                        self.white_char(token_end, ch, spans);
                    }
                }
                '!' => {
                    self.last_token.push(ch);
                    self.drain_buf_colored(token_end, self.theme.key_words, spans);
                }
                '(' => {
                    self.drain_buf_colored(token_end, self.theme.functions, spans);
                    let color = len_to_color(self.brackets.len());
                    self.last_token.push(ch);
                    self.brackets.push(color);
                    self.drain_buf_colored(token_end, color, spans);
                }
                ')' => {
                    let color = self.brackets.pop().unwrap_or(default_color());
                    self.drain_buf(token_end, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(token_end, color, spans);
                }
                '{' => {
                    self.drain_buf(token_end, spans);
                    let color = len_to_color(self.curly.len());
                    self.last_token.push(ch);
                    self.curly.push(color);
                    self.drain_buf_colored(token_end, color, spans);
                }
                '}' => {
                    let color = self.curly.pop().unwrap_or(default_color());
                    self.drain_buf(token_end, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(token_end, color, spans);
                }
                '[' => {
                    self.drain_buf(token_end, spans);
                    let color = len_to_color(self.square.len());
                    self.last_token.push(ch);
                    self.square.push(color);
                    self.drain_buf_colored(token_end, color, spans);
                }
                ']' => {
                    let color = self.square.pop().unwrap_or(default_color());
                    self.drain_buf(token_end, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(token_end, color, spans);
                }
                _ => self.last_token.push(ch),
            }
        }
    }

    fn handled_key_word(&mut self, token_end: usize, spans: &mut Vec<Span>) -> bool {
        if self.key_words.contains(&self.last_token.trim()) {
            self.last_key_words.push(self.last_token.to_owned());
            self.drain_with_select(token_end, self.theme.key_words, spans);
            return true;
        }
        false
    }

    fn handled_object(&mut self, token_end: usize, spans: &mut Vec<Span>) -> bool {
        if let Some(ch) = self.last_token.trim().chars().next() {
            if ch.is_uppercase() {
                self.drain_with_select(token_end, self.theme.class_or_struct, spans);
                return true;
            }
        }
        false
    }

    pub fn syntax_spans<'a>(&mut self, idx: usize, content: &'a str) -> ListItem<'a> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, self.max_digits),
            Style::default().fg(Color::Gray),
        )];
        self.set_select_char_range(idx, content.len());
        self.token_start = 0;
        if self.select_at_line.is_some() && content.is_empty() {
            spans.push(Span {
                content: " ".into(),
                style: Style {
                    bg: Some(self.theme.selected),
                    ..Default::default()
                },
            })
        } else {
            self.process_line(content, &mut spans);
            if !self.last_token.is_empty() {
                self.drain_buf(content.len().checked_sub(1).unwrap_or_default(), &mut spans);
            }
        }
        ListItem::new(Spans::from(spans))
    }

    fn white_char(&mut self, idx: usize, ch: char, spans: &mut Vec<Span>) {
        if matches!(self.select_at_line, Some((from, to)) if from <= idx && idx < to) {
            spans.push(Span::styled(
                String::from(ch),
                Style {
                    bg: Some(self.theme.selected),
                    fg: Some(Color::White),
                    ..Default::default()
                },
            ));
        } else {
            spans.push(Span::styled(
                String::from(ch),
                Style {
                    fg: Some(Color::White),
                    ..Default::default()
                },
            ))
        }
        self.token_start += 1;
    }

    fn drain_buf_object(&mut self, token_end: usize, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) {
            self.drain_with_select(token_end, self.theme.class_or_struct, spans)
        }
    }

    fn drain_buf_colored(&mut self, token_end: usize, color: Color, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) && !self.handled_object(token_end, spans) {
            self.drain_with_select(token_end, color, spans)
        }
    }

    fn drain_buf(&mut self, token_end: usize, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) && !self.handled_object(token_end, spans) {
            self.drain_with_select(token_end, self.theme.default, spans)
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn drain_with_select(&mut self, token_end: usize, color: Color, spans: &mut Vec<Span>) {
        let style = Style {
            fg: Some(color),
            ..Default::default()
        };
        if let Some((select_start, select_end)) = self.select_at_line {
            if select_start <= self.token_start && token_end < select_end {
                spans.push(Span::styled(
                    self.last_token.drain(..).collect::<String>(),
                    style.bg(self.theme.selected),
                ));
            } else if select_end <= self.token_start || token_end <= select_start {
                spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
            } else {
                if select_start <= self.token_start {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_end - self.token_start),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else if self.token_start <= select_start && select_end <= token_end {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_start - self.token_start),
                        style,
                    ));
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_end - select_start),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_start - self.token_start),
                        style,
                    ));
                    spans.push(Span::styled(
                        self.last_token.drain(..).collect::<String>(),
                        style.bg(self.theme.selected),
                    ));
                };
            }
        } else {
            spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
        }
        self.token_start = token_end;
    }
}

fn default_color() -> Color {
    COLORS[COLORS.len() - 1]
}

fn len_to_color(len: usize) -> Color {
    COLORS[len % COLORS.len()]
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = (idx + 1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}

fn drain_token_checked(token: &mut String, last_idx: usize) -> String {
    if last_idx >= token.len() {
        token.drain(..).collect()
    } else {
        token.drain(..last_idx).collect()
    }
}
