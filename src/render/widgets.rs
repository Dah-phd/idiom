use crate::render::{
    backend::{Backend, Style},
    layout::{Line, Rect},
    UTF8Safe,
};

use super::{backend::BackendProtocol, layout::RectIter};

pub enum Word {
    Raw { text: String, utf8_len: usize, width: usize },
    Styled { text: String, utf8_len: usize, width: usize, style: Style },
}

impl From<String> for Word {
    fn from(text: String) -> Self {
        Self::Raw { utf8_len: text.char_len(), width: text.width(), text }
    }
}

impl From<(String, Style)> for Word {
    fn from((text, style): (String, Style)) -> Self {
        Self::Styled { utf8_len: text.char_len(), width: text.width(), text, style }
    }
}

impl From<(Style, String)> for Word {
    fn from((style, text): (Style, String)) -> Self {
        Self::Styled { utf8_len: text.char_len(), width: text.width(), text, style }
    }
}

pub struct StyledLine {
    inner: Vec<Word>,
}

impl StyledLine {
    fn wrap(&self, lines: &mut RectIter, backend: &mut Backend) -> bool {
        let mut current_line = match lines.next() {
            Some(line) => line,
            None => return false,
        };
        let mut remaining = current_line.width;
        backend.go_to(current_line.row, current_line.col);
        for word in self.inner.iter() {
            match word {
                Word::Raw { text, utf8_len, width } => {
                    if text.len() > remaining {
                        let mut end_loc = text.len() - remaining;
                        let (current, mut text_snip) = text.split_at(end_loc);
                        backend.print(current);
                        current_line = match lines.next() {
                            Some(line) => line,
                            None => return false,
                        };
                        remaining = current_line.width;
                        backend.go_to(current_line.row, current_line.col);
                        while text_snip.len() > remaining {
                            end_loc = text_snip.len() - remaining;
                            let (current, next) = text_snip.split_at(end_loc);
                            backend.print(current);
                            text_snip = next;
                            current_line = match lines.next() {
                                Some(line) => line,
                                None => return false,
                            };
                            remaining = current_line.width;
                            backend.go_to(current_line.row, current_line.col);
                        }
                        remaining -= text_snip.width();
                        backend.print(text_snip);
                    } else {
                        remaining -= text.len();
                        backend.print(text);
                    }
                }
                Word::Styled { text, utf8_len, style, width } => {
                    if text.len() > remaining {
                        let mut end_loc = text.len() - remaining;
                        let (current, mut text_snip) = text.split_at(end_loc);
                        backend.print_styled(current, *style);
                        current_line = match lines.next() {
                            Some(line) => line,
                            None => return false,
                        };
                        remaining = current_line.width;
                        backend.go_to(current_line.row, current_line.col);
                        while text_snip.len() > remaining {
                            end_loc = text_snip.len() - remaining;
                            let (current, next) = text_snip.split_at(end_loc);
                            backend.print_styled(current, *style);
                            text_snip = next;
                            current_line = match lines.next() {
                                Some(line) => line,
                                None => return false,
                            };
                            remaining = current_line.width;
                            backend.go_to(current_line.row, current_line.col);
                        }
                        remaining -= text_snip.len();
                        backend.print_styled(text_snip, *style);
                    } else {
                        remaining -= text.len();
                        backend.print_styled(text, *style);
                    }
                }
            }
        }
        true
    }

    fn render(&self, line: Line, backend: &mut Backend) {
        let mut builder = line.unsafe_builder(backend);
        for word in self.inner.iter() {
            if !match word {
                Word::Raw { text, utf8_len, width } => builder.push(text),
                Word::Styled { text, utf8_len, style, width } => builder.push_styled(text, *style),
            } {
                return;
            }
        }
    }

    fn render_rev(&self, line: Line, backend: &mut Backend) {
        let mut builder = line.unsafe_builder_rev(backend);
        for word in self.inner.iter() {
            if !match word {
                Word::Raw { text, utf8_len, width } => builder.push(text),
                Word::Styled { text, utf8_len, style, width } => builder.push_styled(text, *style),
            } {
                return;
            }
        }
    }
}

impl From<Vec<Word>> for StyledLine {
    fn from(inner: Vec<Word>) -> Self {
        Self { inner }
    }
}

#[allow(dead_code)]
pub fn paragraph<'a>(area: Rect, text: impl Iterator<Item = &'a str>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for text_line in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.len() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render(text_slice, backend);
                        } else {
                            line.render(text_line[at_char..].as_ref(), backend);
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return;
                        }
                    }
                } else {
                    line.render(&text_line, backend);
                };
            }
            None => return,
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}

pub fn alt_paragraph_styled<'a>(area: Rect, text: impl Iterator<Item = &'a StyledLine>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for line_text in text {
        if !line_text.wrap(&mut lines, backend) {
            return;
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}

pub fn paragraph_styled<'a>(area: Rect, text: impl Iterator<Item = (&'a str, Style)>, backend: &mut Backend) {
    let mut lines = area.into_iter();
    for (text_line, style) in text {
        match lines.next() {
            Some(mut line) => {
                if text_line.len() > line.width {
                    let mut at_char = 0;
                    let mut remaining = text_line.len();
                    while remaining != 0 {
                        let width = line.width;
                        if let Some(text_slice) = text_line.get(at_char..at_char + width) {
                            line.render_styled(text_slice, style, backend);
                        } else {
                            line.render_styled(text_line[at_char..].as_ref(), style, backend);
                            break;
                        }
                        if let Some(next_line) = lines.next() {
                            line = next_line;
                            at_char += line.width;
                            remaining = remaining.saturating_sub(width);
                        } else {
                            return;
                        }
                    }
                } else {
                    line.render_styled(&text_line, style, backend);
                };
            }
            None => return,
        }
    }
    for remaining_line in lines {
        remaining_line.render_empty(backend);
    }
}
