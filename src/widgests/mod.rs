use ratatui::text::Line;
mod button;
mod list_state;
mod text_field;
pub use button::Button;
pub use list_state::WrappedState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

pub fn centered_rect_static(h: u16, v: u16, rect: Rect) -> Rect {
    let h_diff = rect.width.saturating_sub(h) / 2;
    let v_diff = rect.height.saturating_sub(v) / 2;
    let first_split = Layout::default()
        .constraints([
            Constraint::Length(v_diff),
            Constraint::Min(v),
            Constraint::Length(v_diff),
        ])
        .split(rect);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_diff),
            Constraint::Min(h),
            Constraint::Length(h_diff),
        ])
        .split(first_split[1])[1]
}

pub fn right_corner_rect_static(h: u16, v: u16, rect: Rect) -> Rect {
    Layout::new(Direction::Horizontal, [Constraint::Percentage(100), Constraint::Min(h)])
        .split(Layout::new(Direction::Vertical, [Constraint::Min(v), Constraint::Percentage(100)]).split(rect)[0])[1]
}

pub fn dynamic_cursor_rect_sized_height(
    lines: usize, // min 3
    mut x: u16,
    mut y: u16,
    base: Rect,
) -> Option<Rect> {
    //  ______________
    // |y,x _____     |
    // |   |     |    | base.hight (y)
    // |   |     | h..|
    // |   |     |    |
    // |    -----     |
    // |    width(60) |
    //  --------------
    //   base.width (x)
    //
    let mut height = (lines.min(5) + 2) as u16;
    let mut width = 60;
    if base.height < height + y {
        if base.height > 3 + y {
            height = base.height - y;
        } else if y > 3 && base.height > y {
            // ensures overflowed y's are handled
            let new_y = y.saturating_sub(height + 1);
            height = y - (new_y + 1);
            y = new_y;
        } else {
            return None;
        }
    };
    if base.width < width + x {
        if base.width < 30 + x {
            x = base.width.checked_sub(30)?;
            width = 30;
        } else {
            width = base.width - x;
        }
    };
    Some(Rect { x, y, width, height })
}
