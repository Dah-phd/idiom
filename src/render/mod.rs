mod button;
pub mod layout;
mod list_state;
mod text_field;
use crate::render::layout::Rect;
pub use button::Button;
pub use list_state::WrappedState;
use ratatui::text::Line;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect as RRect},
    style::{Modifier, Style},
    text::Span,
};
use std::borrow::Cow;
pub use text_field::TextField;

pub const LINE_CONTINIUES: Span<'static> =
    Span { content: Cow::Borrowed(">>"), style: Style::new().add_modifier(Modifier::REVERSED) };

pub fn wrapped_line_start(skipped: usize, line: Option<Line<'static>>) -> Line<'static> {
    let mut line = line.unwrap_or_default();
    if skipped == 0 {
        return line;
    };
    line.spans.truncate(1);
    line.spans.push(Span {
        content: format!("..{skipped} hidden wrapped lines").into(),
        style: Style {
            fg: None,
            bg: None,
            add_modifier: Modifier::REVERSED,
            sub_modifier: Modifier::empty(),
            underline_color: None,
        },
    });
    line
}

pub fn count_as_string(len: usize) -> String {
    if len < 10 {
        format!("  {len}")
    } else if len < 100 {
        format!(" {len}")
    } else {
        String::from("99+")
    }
}

pub fn centered_rect_static(cols: u16, rows: u16, rect: RRect) -> RRect {
    let h_diff = rect.width.saturating_sub(cols) / 2;
    let v_diff = rect.height.saturating_sub(rows) / 2;
    let first_split = Layout::default()
        .constraints([
            Constraint::Length(v_diff),
            Constraint::Min(rows),
            Constraint::Length(v_diff),
        ])
        .split(rect);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_diff),
            Constraint::Min(cols),
            Constraint::Length(h_diff),
        ])
        .split(first_split[1])[1]
}

pub fn right_corner_rect_static(cols: u16, rows: u16, rect: RRect) -> RRect {
    Layout::new(Direction::Horizontal, [Constraint::Percentage(100), Constraint::Min(cols)])
        .split(Layout::new(Direction::Vertical, [Constraint::Min(rows), Constraint::Percentage(100)]).split(rect)[0])[1]
}

pub fn dynamic_cursor_rect_sized_height(
    lines: usize, // min 3
    mut col: u16,
    mut row: u16,
    base: RRect,
) -> Option<RRect> {
    let mut height = (lines.min(5) + 2) as u16;
    let mut width = 60;
    if base.height < height + row {
        if base.height > 3 + row {
            height = base.height - row;
        } else if row > 3 && base.height > row {
            // ensures overflowed y's are handled
            let new_y = row.saturating_sub(height + 1);
            height = row - (new_y + 1);
            row = new_y;
        } else {
            return None;
        }
    };
    if base.width < width + col {
        if base.width < 30 + col {
            col = base.width.checked_sub(30)?;
            width = 30;
        } else {
            width = base.width - col;
        }
    };
    Some(RRect { x: col, y: row, width, height })
}

pub fn dynamic_cursor_rect_sized_height_(
    lines: usize, // min 3
    mut col: u16,
    mut row: u16,
    base: Rect,
) -> Option<Rect> {
    let mut height = (lines.min(5) + 2) as u16;
    let mut width = 60;
    if base.height < height + row {
        if base.height > 3 + row {
            height = base.height - row;
        } else if row > 3 && base.height > row {
            // ensures overflowed y's are handled
            let new_y = row.saturating_sub(height + 1);
            height = row - (new_y + 1);
            row = new_y;
        } else {
            return None;
        }
    };
    if base.width < width + col as usize {
        if base.width < 30 + col as usize {
            col = base.width.checked_sub(30)? as u16;
            width = 30;
        } else {
            width = base.width - col as usize;
        }
    };
    Some(Rect::new(row, col, width, height))
}

pub fn dynamic_cursor_rect_bordered(
    lines: usize, // min 3
    mut col: u16,
    mut row: u16,
    base: Rect,
) -> Option<Rect> {
    let mut height = (lines.min(5) + 2) as u16;
    let mut width = 60;
    if base.height < height + row {
        if base.height > 3 + row {
            height = base.height - row;
        } else if row > 3 && base.height > row {
            // ensures overflowed y's are handled
            let new_y = row.saturating_sub(height + 1);
            height = row - (new_y + 1);
            row = new_y;
        } else {
            return None;
        }
    };
    if base.width < width + col as usize {
        if base.width < 30 + col as usize {
            col = base.width.checked_sub(30)? as u16;
            width = 30;
        } else {
            width = base.width - col as usize;
        }
    };
    Some(Rect::new_bordered(row, col, width, height))
}
