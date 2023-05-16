use tui::{widgets::ListItem, text::{Spans, Span}, style::{Color, Style}};


pub fn linter(line:usize, content: &String, max_digits_in_line: u32) -> ListItem {
    ListItem::new(Spans::from(vec![
        Span::styled(format!("{: >4} ", line+1), Style::default().fg(Color::Gray)),
        Span::raw(content)
    ]))
}
