use super::{diagnostics::DiagnosticData, Legend};
use crate::{
    ext_tui::StyleExt,
    workspace::cursor::{Cursor, EncodedWordRange},
    workspace::line::EditorLine,
};
use crossterm::style::ContentStyle;
use lsp_types::SemanticToken;
use unicode_width::UnicodeWidthChar;

/// perform check and reformat on delta to ensure on overlap is happening
pub fn validate_and_format_delta_tokens(tokens: &mut Vec<SemanticToken>) {
    let mut last_len = 0;
    let mut idx = 0;
    loop {
        let Some(token) = tokens.get_mut(idx) else { return };

        if token.delta_line != 0 {
            last_len = 0;
        }

        // drop empty token and extend next delta start
        if token.length == 0 {
            let removed = tokens.remove(idx);
            if let Some(next_token) = tokens.get_mut(idx) {
                if next_token.delta_line != 0 {
                    continue;
                }
                next_token.delta_start += removed.delta_start;
            }
            continue;
        // fix overlapps
        } else if last_len > token.delta_start {
            if token.delta_start == 0 {
                tokens.remove(idx);
                continue;
            } else {
                last_len = token.delta_start;
                if let Some(prev_token) = tokens.get_mut(idx - 1) {
                    prev_token.length = last_len;
                }
            }
        } else {
            last_len = token.length;
        }
        idx += 1;
    }
}

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

#[derive(Default, PartialEq, Debug, Clone)]
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

    pub fn increment_before_encoded(&mut self, mut idx: usize) {
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

    pub fn decrement_at_encoded(&mut self, mut idx: usize) {
        let mut token_iter = self.inner.iter_mut().enumerate();
        while let Some((token_idx, token)) = token_iter.next() {
            if idx < token.delta_start {
                token.delta_start -= 1;
                return;
            }
            if idx < token.delta_start + token.len {
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
            idx -= token.delta_start;
        }
    }

    #[inline]
    pub fn remove_tokens_till(&mut self, mut till: usize) {
        while !self.inner.is_empty() {
            let token = &mut self.inner[0];
            if till <= token.delta_start {
                token.delta_start -= till;
                return;
            }
            if till < token.delta_start + token.len {
                till -= token.delta_start;
                token.len -= till;
                token.delta_start = 0;
                return;
            }
            if till >= token.delta_start + token.len {
                till -= token.delta_start;
                self.inner.remove(0);
                continue;
            }
        }
    }

    #[inline]
    pub fn mark_diagnostics(&mut self, diagnostic: &DiagnosticData) {
        let mut cursor = 0;

        for token in self.inner.iter_mut() {
            cursor += token.delta_start;
            match diagnostic.end {
                Some(end) if diagnostic.start <= cursor && token.len + cursor <= end => {
                    token.style.undercurle(diagnostic.style.foreground_color);
                }
                None if diagnostic.start <= cursor => {
                    token.style.undercurle(diagnostic.style.foreground_color);
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

    /// force word in token line making sure it will fit with other tokens
    pub fn set_encoded_word_checked(&mut self, word: &EncodedWordRange, style: ContentStyle) {
        let mut ranges = vec![];
        let mut at_number = 0;
        // coerce to ranges
        for token in self.inner.drain(..) {
            at_number += token.delta_start;
            ranges.push((at_number, at_number + token.len, token.style));
        }

        todo!();

        // return to delta token
        let mut last_start = 0;
        for (start, end, style) in ranges.into_iter() {
            self.inner.push(Token { len: end - start, delta_start: start - last_start, style });
            last_start = start;
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

#[derive(Debug, PartialEq, Clone)]
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
        let token =
            Token { len: 0, delta_start: text.len().saturating_sub(1) / text_width, style: ContentStyle::default() };
        let tokens = text.tokens_mut_unchecked();
        tokens.clear();
        tokens.push(token);
    } else {
        complex_wrap_calc(text, text_width);
    }
    text.tokens().char_len()
}

pub fn complex_wrap_calc(text: &mut EditorLine, text_width: usize) {
    text.tokens_mut_unchecked().clear();
    let mut counter = text_width;
    let mut wraps = Token { delta_start: 0, len: 0, style: ContentStyle::default() };
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or_default();
        if w > counter {
            counter = text_width;
            wraps.delta_start += 1;
        }
        counter -= w;
    }
    text.tokens_mut_unchecked().push(wraps);
}

pub fn calc_wrap_line_capped(text: &mut EditorLine, cursor: &Cursor) -> Option<usize> {
    let text_width = cursor.text_width;
    let cursor_char = cursor.char;
    let max_rows = cursor.max_rows;
    text.tokens_mut_unchecked().clear();
    if text.is_simple() {
        let token = Token { len: 0, delta_start: text.len() / text_width, style: ContentStyle::default() };
        text.tokens_mut_unchecked().push(token);
        let cursor_at_row = 2 + cursor_char / text_width;
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    } else {
        let mut counter = text_width;
        let mut cursor_at_row = 1;
        let mut prev_idx_break = 0;
        let mut wraps = Token { delta_start: 0, len: 0, style: ContentStyle::default() };
        for (idx, ch) in text.chars().enumerate() {
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
        if counter == 0 {
            wraps.delta_start += 1;
        }
        if prev_idx_break < cursor_char {
            cursor_at_row += 1;
        }
        text.tokens_mut_unchecked().push(wraps);
        if cursor_at_row > max_rows {
            return Some(cursor_at_row - max_rows);
        }
    }
    None
}
