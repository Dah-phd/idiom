use crate::{
    ext_tui::StyleExt,
    global_state::GlobalState,
    workspace::{
        line::{EditorLine, LineContext},
        CursorPosition,
    },
};
use crossterm::style::{ContentStyle, Stylize};
use idiom_tui::{utils::CharLimitedWidths, Backend};
use std::ops::Range;

use super::{WRAP_CLOSE, WRAP_OPEN};

pub fn render(
    line: &mut EditorLine,
    ctx: &mut LineContext,
    line_width: usize,
    cursors: Vec<CursorPosition>,
    selects: Vec<Range<usize>>,
    gs: &mut GlobalState,
) {
    if line_width > line.char_len() {
        basic(line, ctx, cursors, selects, gs);
        if let Some(diagnostics) = line.diagnostics.as_ref() {
            diagnostics.inline_render(line_width - line.char_len(), gs.backend());
        }
    } else {
        self::partial(line, ctx, line_width, cursors, selects, gs);
    }
}

pub fn basic(
    line: &EditorLine,
    ctx: &LineContext,
    cursors: Vec<CursorPosition>,
    selects: Vec<Range<usize>>,
    gs: &mut GlobalState,
) {
    let select_color = gs.theme.selected;
    let backend = gs.backend();
    let char_position = ctx.lexer.char_lsp_pos;
    let mut reset_style = ContentStyle::default();
    let mut tokens = line.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;
    let mut idx = 0;

    let mut cursor_iter = cursors.into_iter().map(|c| c.char);
    let mut cursor_idx = cursor_iter.next().unwrap_or(usize::MAX);

    let mut select_iter = selects.into_iter();
    let mut select = select_iter.next().unwrap_or_default();

    if let Some(token) = tokens.next() {
        if token.delta_start == 0 {
            backend.set_style(token.style);
            counter = token.len;
        } else {
            lined_up.replace(token.style);
            counter = token.delta_start;
        }
        last_len = token.len;
    };

    for text in line.chars() {
        if select.start == idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }
        if select.end == idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
            if let Some(new_select) = select_iter.next() {
                select = new_select;
                if select.start == idx {
                    backend.set_bg(Some(select_color));
                    reset_style.set_bg(Some(select_color));
                }
            }
        }

        if counter == 0 {
            match lined_up.take() {
                None => match tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start - (last_len + 1);
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len - 1;
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len - 1;
                }
            }
        } else {
            counter = counter.saturating_sub(char_position(text));
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed());
            if let Some(new_cursor_idx) = cursor_iter.next() {
                cursor_idx = new_cursor_idx;
            }
        } else {
            backend.print(text);
        }

        idx += 1;
    }
    if idx <= cursor_idx && cursor_idx != usize::MAX {
        backend.print_styled(" ", ContentStyle::reversed());
    }
    backend.reset_style();
}

pub fn partial(
    code: &mut EditorLine,
    ctx: &mut LineContext,
    mut line_width: usize,
    cursors: Vec<CursorPosition>,
    selects: Vec<Range<usize>>,
    gs: &mut GlobalState,
) {
    let backend = &mut gs.backend;
    let last_idx = cursors.last().map(|c| c.char).unwrap_or_default();

    let char_position = ctx.lexer.char_lsp_pos;

    // index needs to be generated based on 0 skipped chars on multicursor
    // skipped chars are use to store info on multi cursor
    let skipped = code.cached.skipped_chars();
    code.cached.set_skipped_chars(0);
    let mut idx = code.generate_skipped_chars_complex(last_idx, line_width);
    code.cached.set_skipped_chars(skipped);

    let mut content = CharLimitedWidths::new(code.as_str(), 3);

    let mut cursor_iter = cursors.into_iter().map(|x| x.char);
    let mut cursor_idx = cursor_iter.next().unwrap_or(usize::MAX);

    let mut select_iter = selects.into_iter();
    let mut select = select_iter.next().unwrap_or_default();

    let mut cursor = 0;
    for _ in 0..idx {
        cursor += content.next().map(|(ch, ..)| char_position(ch)).unwrap_or_default();
    }

    let select_color = gs.theme.selected;
    let mut reset_style = ContentStyle::default();
    if select.start <= idx && idx < select.end {
        reset_style.set_bg(Some(select_color));
        backend.set_bg(Some(select_color));
    }

    let mut tokens = code.iter_tokens();
    let mut counter = 0;
    let mut last_len = 0;
    let mut lined_up = None;

    for token in tokens.by_ref() {
        if token.delta_start + token.len > cursor {
            last_len = token.len;
            if token.delta_start > cursor {
                counter = token.delta_start - cursor;
                lined_up.replace(token.style);
            } else {
                backend.update_style(token.style);
                counter = (token.delta_start + last_len) - cursor;
            }
            break;
        }
        cursor -= token.delta_start;
    }

    if idx != 0 {
        let mut skipped_cursors = false;
        while cursor_idx < idx {
            cursor_idx = cursor_iter.next().unwrap_or(usize::MAX);
            skipped_cursors = true;
        }
        let style = match skipped_cursors {
            true => ContentStyle::reversed(),
            false => ctx.accent_style.reverse(),
        };
        backend.print_styled(WRAP_OPEN, style);
        line_width -= 1;
    };

    for (text, char_width) in content {
        if select.start == idx {
            backend.set_bg(Some(select_color));
            reset_style.set_bg(Some(select_color));
        }

        if select.end == idx {
            backend.set_bg(None);
            reset_style.set_bg(None);
            if let Some(new_select) = select_iter.next() {
                select = new_select;
                if select.start == idx {
                    backend.set_bg(Some(select_color));
                    reset_style.set_bg(Some(select_color));
                }
            }
        }

        if counter == 0 {
            match lined_up.take() {
                Some(style) => {
                    backend.update_style(style);
                    counter = last_len.saturating_sub(char_position(text));
                }
                None => match tokens.next() {
                    None => {
                        backend.set_style(reset_style);
                        counter = usize::MAX;
                    }
                    Some(token) => {
                        if token.delta_start > last_len {
                            counter = token.delta_start.saturating_sub(last_len + char_position(text));
                            lined_up.replace(token.style);
                            backend.set_style(reset_style);
                        } else {
                            counter = token.len.saturating_sub(char_position(text));
                            backend.update_style(token.style);
                        }
                        last_len = token.len;
                    }
                },
            }
        } else {
            counter = counter.saturating_sub(char_position(text));
        }

        if char_width > line_width {
            break;
        } else {
            line_width -= char_width;
        }

        if cursor_idx == idx {
            backend.print_styled(text, ContentStyle::reversed());
            if let Some(new_cursor_idx) = cursor_iter.next() {
                cursor_idx = new_cursor_idx;
            }
        } else {
            backend.print(text);
        }
        idx += 1;
    }

    backend.reset_style();
    if idx <= cursor_idx {
        backend.print_styled(" ", ContentStyle::reversed());
    } else if code.char_len() > idx {
        backend.print_styled(WRAP_CLOSE, ctx.accent_style.reverse());
    }
}
