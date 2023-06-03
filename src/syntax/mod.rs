mod theme;
use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

use crate::messages::FileType;

use self::theme::Theme;

pub const COLORS: [Color; 3] = [Color::Magenta, Color::Blue, Color::Yellow];

#[derive(Debug)]
pub struct Lexer {
    select: Option<((usize, usize), (usize, usize))>,
    token_start: usize,
    select_at_line: Option<(usize, usize)>,
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    last_token: String,
    key_words: Vec<&'static str>,
    last_key_words: Vec<String>,
    theme: Theme,
    max_digits: usize,
}

impl Default for Lexer {
    fn default() -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut"],
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
    pub fn from_type(file_type: &FileType) -> Self {
        #[allow(clippy::match_single_binding)]
        match file_type {
            _ => Self::default(),
        }
    }

    pub fn max_line_digits_from(&mut self, len: usize) {
        self.max_digits = len
    }

    fn set_select_char_range(&mut self, at_line: usize, max_len: usize) {
        if let Some(((from_line, from_char), (to_line, to_char))) = self.select {
            if from_line > at_line || at_line > to_line {
                self.select_at_line = None;
            } else if from_line < at_line && at_line < to_line {
                self.select_at_line = Some((0, max_len));
            } else if from_line == at_line && at_line == to_line {
                self.select_at_line = Some((from_char, to_char));
            } else if from_line == at_line {
                self.select_at_line = Some((from_char, max_len));
            } else if to_line == at_line {
                self.select_at_line = Some((0, to_char))
            }
        }
    }

    fn process_line(&mut self, content: &str, spans: &mut Vec<Span>) {
        if content.starts_with("mod") {}
        if content.starts_with("use") {}
        let char_stream = content.chars().enumerate();
        for (idx, ch) in char_stream {
            match ch {
                ' ' => {
                    self.drain_buf(idx, spans);
                    self.last_token.push(ch);
                }
                '.' => {
                    self.drain_buf(idx, spans);
                    self.white_char(idx, ch, spans);
                }
                '<' => {
                    self.drain_buf(idx, spans);
                    self.white_char(idx, ch, spans)
                }
                '>' => {
                    self.drain_buf(idx, spans);
                    self.white_char(idx, ch, spans)
                }
                '(' => {
                    self.drain_buf_colored(idx, self.theme.function, spans);
                    let color = len_to_color(Some(self.brackets.len()));
                    self.last_token.push(ch);
                    self.brackets.push(color);
                    self.drain_buf_colored(idx, color, spans);
                }
                ')' => {
                    let color = if let Some(color) = self.brackets.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    self.drain_buf(idx, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(idx, color, spans);
                }
                '{' => {
                    self.drain_buf(idx, spans);
                    let color = len_to_color(Some(self.curly.len()));
                    self.last_token.push(ch);
                    self.curly.push(color);
                    self.drain_buf_colored(idx, color, spans);
                }
                '}' => {
                    let color = if let Some(color) = self.curly.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    self.drain_buf(idx, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(idx, color, spans);
                }
                '[' => {
                    self.drain_buf(idx, spans);
                    let color = len_to_color(Some(self.square.len()));
                    self.last_token.push(ch);
                    self.square.push(color);
                    self.drain_buf_colored(idx, color, spans);
                }
                ']' => {
                    let color = if let Some(color) = self.square.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    self.drain_buf(idx, spans);
                    self.last_token.push(ch);
                    self.drain_buf_colored(idx, color, spans);
                }
                ':' => {
                    self.drain_buf(idx, spans);
                    self.white_char(idx, ch, spans)
                }
                _ => self.last_token.push(ch),
            }
        }
    }

    pub fn select(&mut self, range: (&(usize, usize), &(usize, usize))) {
        self.select = Some((*range.0, *range.1))
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme
    }

    fn handled_key_word(&mut self, idx: usize, spans: &mut Vec<Span>) -> bool {
        if self.key_words.contains(&self.last_token.trim()) {
            self.last_key_words.push(self.last_token.to_owned());
            self.drain_with_select(idx, self.theme.kword, spans);
            return true;
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
        if matches!(self.select_at_line, Some((from, to)) if from <= idx && idx <= to) {
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

    fn drain_buf_colored(&mut self, idx: usize, color: Color, spans: &mut Vec<Span>) {
        if !self.handled_key_word(idx, spans) {
            self.drain_with_select(idx, color, spans)
        }
    }

    fn drain_buf(&mut self, idx: usize, spans: &mut Vec<Span>) {
        if !self.handled_key_word(idx, spans) {
            self.drain_with_select(idx, self.theme.default, spans)
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn drain_with_select(&mut self, token_end: usize, color: Color, spans: &mut Vec<Span>) {
        let style = Style {
            fg: Some(color),
            ..Default::default()
        };
        if let Some((select_start, select_end)) = self.select_at_line {
            if select_start <= self.token_start && token_end <= select_end {
                spans.push(Span::styled(
                    self.last_token.drain(..).collect::<String>(),
                    style.bg(self.theme.selected),
                ));
            } else if select_end <= self.token_start || token_end <= select_start {
                spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
            } else {
                if select_start <= self.token_start {
                    spans.push(Span::styled(
                        self.last_token
                            .drain(..(select_end - self.token_start))
                            .collect::<String>(),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else if self.token_start <= select_start && select_end <= token_end {
                    spans.push(Span::styled(
                        self.last_token
                            .drain(..(select_start - self.token_start))
                            .collect::<String>(),
                        style,
                    ));
                    spans.push(Span::styled(
                        self.last_token.drain(..(select_end - select_start)).collect::<String>(),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else {
                    spans.push(Span::styled(
                        self.last_token
                            .drain(..(select_start - self.token_start))
                            .collect::<String>(),
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

fn len_to_color(len: Option<usize>) -> Color {
    if let Some(len) = len {
        COLORS[len % COLORS.len()]
    } else {
        COLORS[COLORS.len() - 1]
    }
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = (idx + 1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}
