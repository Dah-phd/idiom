use super::{diagnostics::DiagnosticData, Legend};
use crate::{render::backend::StyleExt, workspace::cursor::Cursor, workspace::line::EditorLine};
use crossterm::style::ContentStyle;
use lsp_types::SemanticToken;
use unicode_width::UnicodeWidthChar;

pub fn set_tokens(tokens: Vec<SemanticToken>, legend: &Legend, content: &mut [EditorLine]) {
    let mut tokens = tokens.into_iter();

    let token = match tokens.next() {
        Some(token) => token,
        None => return,
    };
    let mut line_idx = token.delta_line as usize;
    let mut token_line = content[line_idx].tokens_mut();
    token_line.clear();
    token_line.push(Token::parse(token, legend));

    for token in tokens {
        if token.delta_line != 0 {
            line_idx += token.delta_line as usize;
            token_line = content[line_idx].tokens_mut();
            token_line.clear();
        };
        token_line.push(Token::parse(token, legend));
    }
}

pub fn set_tokens_partial(tokens: Vec<SemanticToken>, max_lines: usize, legend: &Legend, content: &mut [EditorLine]) {
    let mut tokens = tokens.into_iter();

    let token = match tokens.next() {
        Some(token) => token,
        None => return,
    };
    let mut line_idx = token.delta_line as usize;
    if line_idx > max_lines {
        return;
    }
    let mut token_line = content[line_idx].tokens_mut();
    token_line.clear();
    token_line.push(Token::parse(token, legend));

    for token in tokens {
        if token.delta_line != 0 {
            line_idx += token.delta_line as usize;
            if line_idx > max_lines {
                return;
            }
            token_line = content[line_idx].tokens_mut();
            token_line.clear();
        };
        token_line.push(Token::parse(token, legend));
    }
}

#[derive(Default, PartialEq, Debug)]
pub struct TokenLine {
    inner: Vec<Token>,
}

impl TokenLine {
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn char_len(&self) -> usize {
        self.inner.iter().map(|token| token.delta_start).sum::<usize>()
            + self.inner.last().map(|t| t.len).unwrap_or_default()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn increment_at(&mut self, mut idx: usize) {
        let mut token_iter = self.inner.iter_mut();
        while let Some(token) = token_iter.next() {
            if idx <= token.delta_start {
                token.delta_start += 1;
                return;
            };
            if idx <= token.delta_start + token.len {
                token.len += 1;
                if let Some(next_token) = token_iter.next() {
                    next_token.delta_start += 1;
                }
                return;
            }
            idx -= token.delta_start;
        }
    }

    pub fn decrement_at(&mut self, mut char_idx: usize) {
        let mut token_iter = self.inner.iter_mut().enumerate();
        while let Some((token_idx, token)) = token_iter.next() {
            if char_idx <= token.delta_start {
                token.delta_start -= 1;
                return;
            }
            if char_idx <= token.delta_start + token.len {
                match token.len > 1 {
                    true => {
                        if let Some((.., next_token)) = token_iter.next() {
                            next_token.delta_start -= 1;
                        }
                        token.len -= 1;
                    }
                    // should allways be 1, considering lsp working normally
                    false => {
                        let start_offset = self.inner.remove(token_idx).delta_start;
                        if let Some(next_token) = self.inner.get_mut(token_idx) {
                            next_token.delta_start += start_offset;
                            next_token.delta_start -= 1;
                        }
                    }
                }
                return;
            }
            char_idx -= token.delta_start;
        }
    }

    #[inline]
    pub fn mark_diagnostics(&mut self, diagnostic: &DiagnosticData) {
        let mut cursor = 0;

        for token in self.inner.iter_mut() {
            cursor += token.delta_start;
            match diagnostic.end {
                Some(end) if diagnostic.start <= cursor && token.len + cursor <= end => {
                    token.style.undercurle(Some(diagnostic.color));
                }
                None if diagnostic.start <= cursor => {
                    token.style.undercurle(Some(diagnostic.color));
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn drop_diagnostics(&mut self) {
        for token in self.inner.iter_mut() {
            token.drop_diagstic();
        }
    }

    pub fn push(&mut self, token: Token) {
        self.inner.push(token);
    }

    pub fn insert(&mut self, index: usize, token: Token) {
        self.inner.insert(index, token);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Token> {
        self.inner.iter()
    }
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub len: usize,
    pub delta_start: usize,
    pub style: ContentStyle,
}

impl Token {
    pub fn parse(token: SemanticToken, legend: &Legend) -> Self {
        let SemanticToken { delta_start, length, token_type, token_modifiers_bitset, .. } = token;
        let style = ContentStyle::fg(legend.parse_to_color(token_type as usize, token_modifiers_bitset));
        Self { delta_start: delta_start as usize, len: length as usize, style }
    }

    pub fn drop_diagstic(&mut self) {
        self.style.reset_mods();
    }
}

/// In plain text condition TokenLine is used to store wrapped lines, without affecting the code editing
pub fn calc_wraps(content: &mut [EditorLine], text_width: usize) {
    for text in content.iter_mut() {
        calc_wrap_line(text, text_width);
    }
}

pub fn calc_wrap_line(text: &mut EditorLine, text_width: usize) -> usize {
    if text.is_simple() {
        text.tokens.clear();
        text.tokens.push(Token {
            len: 0,
            delta_start: text.content.len() / text_width,
            style: ContentStyle::default(),
        });
    } else {
        complex_wrap_calc(text, text_width);
    }
    text.tokens.char_len()
}

pub fn complex_wrap_calc(text: &mut EditorLine, text_width: usize) {
    text.tokens.clear();
    let mut counter = text_width;
    let mut wraps = Token { delta_start: 0, len: 0, style: ContentStyle::default() };
    for ch in text.content.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or_default();
        if w > counter {
            counter = text_width;
            wraps.delta_start += 1;
        }
        counter -= w;
    }
    text.tokens.push(wraps);
}

pub fn calc_wrap_line_capped(text: &mut EditorLine, cursor: &Cursor) -> Option<usize> {
    let text_width = cursor.text_width;
    let cursor_char = cursor.char;
    let max_rows = cursor.max_rows;
    text.tokens.clear();
    if text.is_simple() {
        text.tokens.push(Token {
            len: 0,
            delta_start: text.content.len() / text_width,
            style: ContentStyle::default(),
        });
        let cursor_at_row = 2 + cursor_char / text_width;
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    } else {
        let mut counter = text_width;
        let mut cursor_at_row = 1;
        let mut prev_idx_break = 0;
        let mut wraps = Token { delta_start: 0, len: 0, style: ContentStyle::default() };
        for (idx, ch) in text.content.chars().enumerate() {
            let w = UnicodeWidthChar::width(ch).unwrap_or_default();
            if w > counter {
                counter = text_width;
                wraps.delta_start += 1;
                if prev_idx_break < cursor_char {
                    cursor_at_row += 1;
                }
                prev_idx_break = idx;
            }
            counter -= w;
        }
        text.tokens.push(wraps);
        if prev_idx_break < cursor_char {
            cursor_at_row += 1;
        }
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    }
    None
}
