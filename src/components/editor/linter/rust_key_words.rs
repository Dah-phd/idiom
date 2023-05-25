use super::{theme::Theme, Linter, COLORS};
use tui::{
    style::{Color, Style},
    text::Span,
};

pub struct RustSyntax {
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    last_token: String,
    key_words: Vec<&'static str>,
    last_key_words: Vec<String>,
    theme: Theme,
}

impl Default for RustSyntax {
    fn default() -> Self {
        Self {
            key_words: vec!["pub", "fn", "struct", "use", "mod", "let", "self", "mut"],
            curly: vec![],
            brackets: vec![],
            square: vec![],
            last_token: String::default(),
            last_key_words: vec![],
            theme: Theme::default(),
        }
    }
}

impl Linter for RustSyntax {
    fn get_token_buffer(&mut self) -> &mut String {
        &mut self.last_token
    }

    fn get_theme(&self) -> &Theme {
        &self.theme
    }

    fn process_line(&mut self, content: &str, spans: &mut Vec<Span>) {
        if content.starts_with("mod") {}
        if content.starts_with("use") {}
        let char_stream = content.chars();
        for ch in char_stream {
            match ch {
                ' ' => {
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                }
                '.' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch));
                }
                '<' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                '>' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                '(' => {
                    spans.push(self.drain_buf_colored(self.theme.function));
                    let color = len_to_color(Some(self.brackets.len()));
                    self.last_token.push(ch);
                    self.brackets.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                ')' => {
                    let color = if let Some(color) = self.brackets.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                '{' => {
                    spans.push(self.drain_buf());
                    let color = len_to_color(Some(self.curly.len()));
                    self.last_token.push(ch);
                    self.curly.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                '}' => {
                    let color = if let Some(color) = self.curly.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                '[' => {
                    spans.push(self.drain_buf());
                    let color = len_to_color(Some(self.square.len()));
                    self.last_token.push(ch);
                    self.square.push(color);
                    spans.push(self.drain_buf_colored(color));
                }
                ']' => {
                    let color = if let Some(color) = self.square.pop() {
                        color
                    } else {
                        len_to_color(None)
                    };
                    spans.push(self.drain_buf());
                    self.last_token.push(ch);
                    spans.push(self.drain_buf_colored(color));
                }
                ':' => {
                    spans.push(self.drain_buf());
                    spans.push(self.white_char(ch))
                }
                _ => self.last_token.push(ch),
            }
        }
    }

    fn handled_key_word(&mut self) -> Option<Span<'static>> {
        if self.key_words.contains(&self.last_token.trim()) {
            self.last_key_words.push(self.last_token.to_owned());
            Some(Span::styled(
                self.last_token.drain(..).collect::<String>(),
                Style {
                    fg: Some(self.theme.kword),
                    ..Default::default()
                },
            ))
        } else {
            None
        }
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
    let mut as_str = idx.to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}
