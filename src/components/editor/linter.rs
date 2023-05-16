use tui::{
    style::{Color, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

const WHITE_SPACE: char = ' ';

pub fn linter(idx: usize, content: &String, max_digits: usize) -> ListItem {
    ListItem::new(Spans::from(vec![
        Span::styled(get_line_num(idx, max_digits), Style::default().fg(Color::Gray)),
        Span::raw(content),
    ]))
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = idx.to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, WHITE_SPACE)
    }
    as_str.push(WHITE_SPACE);
    as_str
}
