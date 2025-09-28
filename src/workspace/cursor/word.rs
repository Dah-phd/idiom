use super::{CharRange, CursorPosition, Select};
use crate::workspace::EditorLine;
use idiom_tui::UTF8Safe;

/// owns text and location
pub struct PositionedWord {
    range: WordRange,
    text: String,
}

impl PositionedWord {
    pub fn find_at(content: &[EditorLine], position: CursorPosition) -> Option<Self> {
        let range = WordRange::find_at(content, position)?;
        let text = content[range.line][range.from..range.to].to_owned();
        Some(Self { range, text })
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    #[inline]
    pub fn char_len(&self) -> usize {
        self.range.to - self.range.from
    }

    #[inline]
    pub fn range(&self) -> &WordRange {
        &self.range
    }

    pub fn iter_word_ranges<'a, B>(&'a self, content_iter: B) -> impl Iterator<Item = WordRange> + use<'a, B>
    where
        B: Iterator<Item = (usize, &'a EditorLine)>,
    {
        content_iter.flat_map(move |(line, text)| {
            text.as_str().match_indices(self.as_str()).flat_map(move |(position, _)| {
                let prefix = &text.as_str()[..position];
                if prefix.chars().next_back().map(is_word_char).unwrap_or_default() {
                    return None;
                }
                let end_char_idx = position + self.len();
                if text.as_str()[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
                    return None;
                };
                if text.is_simple() {
                    return Some(WordRange { line, from: position, to: end_char_idx });
                }
                let char = prefix.char_len();
                Some(WordRange { line, from: char, to: char + self.char_len() })
            })
        })
    }

    pub fn find_word_inline_after<'a>(
        &'a self,
        content: &'a [EditorLine],
    ) -> Option<impl Iterator<Item = WordRange> + use<'a>> {
        let text = content.get(self.range.line)?;
        let skipped = text.get_to(self.range.to)?;
        let char_before_heystack = skipped.chars().next_back();
        let heystack = &text.as_str()[skipped.len()..];
        Some(heystack.match_indices(self.as_str()).flat_map(move |(position, _)| {
            let prefix = &heystack[..position];
            let prev_char = if position == 0 { char_before_heystack } else { prefix.chars().next_back() };
            if prev_char.map(is_word_char).unwrap_or_default() {
                return None;
            };
            let end_char_idx = position + self.len();
            if heystack[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
                return None;
            };
            if text.is_simple() {
                return Some(WordRange {
                    line: self.range.line,
                    from: self.range.to + position,
                    to: self.range.to + end_char_idx,
                });
            }
            let from = self.range.to + prefix.char_len();
            Some(WordRange { line: self.range.line, from, to: from + self.char_len() })
        }))
    }

    pub fn find_word_inline_before<'a>(
        &'a self,
        content: &'a [EditorLine],
    ) -> Option<impl Iterator<Item = WordRange> + use<'a>> {
        let text = content.get(self.range.line)?;
        let heystack = text.get_to(self.range.from)?;
        Some(heystack.match_indices(self.as_str()).flat_map(move |(position, _)| {
            let prefix = &heystack[..position];
            if prefix.chars().next_back().map(is_word_char).unwrap_or_default() {
                return None;
            };
            let end_char_idx = position + self.len();
            if text.as_str()[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
                return None;
            };
            if text.is_simple() {
                return Some(WordRange { line: self.range.line, from: position, to: end_char_idx });
            }
            let char = prefix.char_len();
            Some(WordRange { line: self.range.line, from: char, to: char + self.char_len() })
        }))
    }
}

/// word location
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WordRange {
    pub line: usize,
    pub from: usize,
    pub to: usize,
}

impl WordRange {
    pub fn find_at(content: &[EditorLine], position: CursorPosition) -> Option<Self> {
        let line = &content[position.line];
        let idx = position.char;
        let mut token_start = 0;
        let mut last_not_in_token = false;
        for (char_idx, ch) in line.chars().enumerate() {
            if is_word_char(ch) {
                if last_not_in_token {
                    token_start = char_idx;
                }
                last_not_in_token = false;
            } else if char_idx >= idx {
                if last_not_in_token {
                    return None;
                }
                return Some(Self { line: position.line, from: token_start, to: char_idx });
            } else {
                last_not_in_token = true;
            }
        }
        if idx < line.char_len() {
            Some(Self { line: position.line, from: token_start, to: line.char_len() })
        } else if !last_not_in_token && token_start <= idx {
            Some(Self { line: position.line, from: token_start, to: idx })
        } else {
            None
        }
    }

    pub fn find_text_at(content: &[EditorLine], position: CursorPosition) -> Option<&str> {
        let range = Self::find_at(content, position)?;
        Some(&content[range.line][range.from..range.to])
    }

    pub fn find_char_range(line: &EditorLine, idx: usize) -> Option<CharRange> {
        let mut token_start = 0;
        let mut last_not_in_token = false;
        for (char_idx, ch) in line.chars().enumerate() {
            if is_word_char(ch) {
                if last_not_in_token {
                    token_start = char_idx;
                }
                last_not_in_token = false;
            } else if char_idx >= idx {
                if last_not_in_token {
                    return None;
                }
                return Some(CharRange { from: token_start, to: char_idx });
            } else {
                last_not_in_token = true;
            }
        }
        if idx < line.char_len() {
            Some(CharRange { from: token_start, to: line.char_len() })
        } else if !last_not_in_token && token_start <= idx {
            Some(CharRange { from: token_start, to: idx })
        } else {
            None
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn get_text<'a>(&self, content: &'a [EditorLine]) -> Option<&'a str> {
        content[self.line].get(self.from, self.to)
    }

    #[allow(dead_code)]
    #[inline]
    pub fn get_text_uncheded<'a>(&self, content: &'a [EditorLine]) -> &'a str {
        &content[self.line][self.from..self.to]
    }

    pub fn as_select(&self) -> Select {
        (CursorPosition { line: self.line, char: self.from }, CursorPosition { line: self.line, char: self.to })
    }
}

#[inline]
fn is_word_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')
}
