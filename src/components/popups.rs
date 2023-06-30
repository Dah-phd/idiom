use crossterm::event::KeyEvent;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, Default)]
pub struct Popup {
    // buttons: Option<Vec<Button>>,
    input: Option<String>,
    size: (usize, usize),
}

impl Popup {
    pub fn render(&mut self, frame: &mut Frame<impl Backend>) {
        let size = frame.size();

        let block = Block::default().title("Popup").borders(Borders::ALL);
        let area = centered_rect(60, 20, size);
        frame.render_widget(Clear, area); //this clears out the background
        frame.render_widget(block, area);
    }

    pub fn map(&mut self, key: &KeyEvent) -> bool {
        false
    }

    fn finish(&mut self) {}
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
