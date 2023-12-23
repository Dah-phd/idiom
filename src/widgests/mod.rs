mod button;
mod text_field;
pub use button::Button;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
pub use text_field::TextField;

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
    let h_diff = rect.width.checked_sub(h).unwrap_or_default() / 2;
    let v_diff = rect.height.checked_sub(v).unwrap_or_default() / 2;
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
