use super::PopupInterface;
use crate::{
    global_state::{Clipboard, PopupMessage, TreeEvent},
    tree::Tree,
    widgests::centered_rect_static,
    widgests::{TextField, WrappedState},
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};
use std::path::PathBuf;

const SELECTOR_CONSTRAINTS: [Constraint; 2] = [Constraint::Min(3), Constraint::Percentage(100)];

pub struct ActivePathSearch {
    options: Vec<PathBuf>,
    state: WrappedState,
    pattern: TextField<PopupMessage>,
}

impl ActivePathSearch {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            options: Vec::new(),
            state: WrappedState::default(),
            pattern: TextField::with_tree_access(String::new()),
        })
    }
}

impl PopupInterface for ActivePathSearch {
    fn key_map(&mut self, key: &KeyEvent, clipbard: &mut Clipboard) -> PopupMessage {
        if let Some(msg) = self.pattern.map(key, clipbard) {
            return msg;
        }
        match key.code {
            KeyCode::Up => self.state.prev(&self.options),
            KeyCode::Down => self.state.next(&self.options),
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
        let split_areas = Layout::new(Direction::Vertical, SELECTOR_CONSTRAINTS).split(area);
        frame.render_widget(
            self.pattern
                .widget()
                .block(Block::new().borders(Borders::ALL).title("Search pattern (Tab to switch to in File search)")),
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
            .block(Block::new().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, split_areas[1], self.state.get());
    }

    fn update_tree(&mut self, file_tree: &mut Tree) {
        if self.pattern.text.is_empty() {
            self.options.clear();
        } else {
            self.options = file_tree.search_paths(&self.pattern.text);
        };
        self.state.drop();
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
