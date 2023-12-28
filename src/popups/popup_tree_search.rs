use super::PopupInterface;
use crate::{
    global_state::{Clipboard, PopupMessage, TreeEvent},
    tree::Tree,
    widgests::centered_rect_static,
    widgests::TextField,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::path::PathBuf;

#[derive(Default)]
pub struct ActiveTreeSearch {
    options: Vec<PathBuf>,
    state: ListState,
    pattern: TextField,
}

impl ActiveTreeSearch {
    pub fn new() -> Box<Self> {
        Box::new(Self { options: Vec::new(), state: ListState::default(), pattern: TextField::with_tree_access() })
    }

    fn next(&mut self) {
        match self.state.selected() {
            Some(idx) => {
                let idx = idx + 1;
                self.state.select(Some(if idx < self.options.len() { idx } else { 0 }));
            }
            None if !self.options.is_empty() => self.state.select(Some(0)),
            _ => (),
        }
    }

    fn prev(&mut self) {
        match self.state.selected() {
            Some(idx) => self.state.select(Some(if idx == 0 { self.options.len() - 1 } else { idx - 1 })),
            None => self.state.select(Some(self.options.len() - 1)),
        }
    }
}

impl PopupInterface for ActiveTreeSearch {
    fn key_map(&mut self, key: &KeyEvent, clipbard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipbard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Tab => return PopupMessage::Tree(TreeEvent::SearchFiles(self.pattern.text.to_owned())),
            KeyCode::Enter => {
                return match self.state.selected() {
                    Some(idx) if !self.options.is_empty() => TreeEvent::Open(self.options.remove(idx)).into(),
                    _ => PopupMessage::Clear,
                }
            }
            _ => {}
        }
        PopupMessage::None
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = centered_rect_static(120, 20, frame.size());
        frame.render_widget(Clear, area);
        let split_areas =
            Layout::new(Direction::Vertical, [Constraint::Min(3), Constraint::Percentage(100)]).split(area);
        frame.render_widget(
            self.pattern.widget().block(Block::default().borders(Borders::ALL).title("Search pattern ")),
            split_areas[0],
        );

        let options = if self.options.is_empty() {
            vec![ListItem::new("No results found!")]
        } else {
            self.options
                .iter()
                .map(|el| ListItem::new(marked_pat_span(&el.display().to_string(), &self.pattern.text)))
                .collect::<Vec<_>>()
        };
        let list = List::new(options)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, split_areas[1], &mut self.state);
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = file_tree.search_paths(&self.pattern.text);
        };
        self.state.select(None);
    }
}

fn marked_pat_span<'a>(option: &'a str, pat: &'a str) -> Line<'static> {
    let mut v = Vec::new();
    let mut from = 0;
    for (idx, _) in option.match_indices(pat) {
        v.push(Span::styled(option[from..idx].to_owned(), Style { add_modifier: Modifier::DIM, ..Default::default() }));
        from = idx + pat.len();
        v.push(Span::styled(
            option[idx..from].to_owned(),
            Style { add_modifier: Modifier::BOLD, ..Default::default() },
        ));
    }
    v.push(Span::styled(option[from..].to_owned(), Style { add_modifier: Modifier::DIM, ..Default::default() }));
    Line::from(v)
}
