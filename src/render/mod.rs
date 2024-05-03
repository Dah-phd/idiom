pub mod backend;
mod button;
pub mod layout;
mod list_state;
pub mod state;
mod text_field;
pub mod widgets;
pub use button::Button;
pub use list_state::WrappedState;
use ratatui::text::Line;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect as RRect},
    style::{Modifier, Style},
    text::Span,
};
pub use text_field::TextField;

/// This can easily gorow to be a framework itself

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
