use tui::{widgets::ListItem, text::{Spans, Span}, style::{Color, Style}};


pub fn linter(code_line: (usize, &String)) -> ListItem {
    let (line, content) = code_line;
    ListItem::new(Spans::from(vec![
        Span::styled(format!("{: >3} ", line), Style::default().fg(Color::Gray)),
        Span::raw(content)
    ]))
}
