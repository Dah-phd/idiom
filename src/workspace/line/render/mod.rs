pub mod ascii_cursor;
pub mod ascii_line;
pub mod complex_cursor;
pub mod complex_line;
use unicode_width::UnicodeWidthChar;

use crate::{
    render::backend::{Backend, BackendProtocol, Style},
    syntax::DiagnosticLine,
};

use super::EditorLine;

#[inline(always)]
pub fn inline_diagnostics(max_len: usize, diagnostics: &Option<DiagnosticLine>, backend: &mut Backend) {
    if let Some(data) = diagnostics.as_ref().and_then(|d| d.data.first()) {
        backend.print_styled(data.truncated_inline(max_len), Style::fg(data.color));
    };
}

#[inline(always)]
pub fn is_wider_complex(line: &impl EditorLine, max_width: usize) -> bool {
    let mut current_with = 0;
    for ch in line.chars() {
        if let Some(char_width) = UnicodeWidthChar::width(ch) {
            current_with += char_width;
            if current_with > max_width {
                return true;
            }
        }
    }
    false
}
