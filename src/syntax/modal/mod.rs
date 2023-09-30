mod parser;
use crate::components::editor::CursorPosition;
use crossterm::event::KeyEvent;
use lsp_types::CompletionItem;
pub use parser::LSPResponseType;
use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};
use std::io::Stdout;

pub trait Modal {
    fn map(&mut self, key: &KeyEvent);
    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16);
}

#[derive(Debug)]
pub struct AutoComplete {
    cursor: CursorPosition,
    completions: Vec<CompletionItem>,
}

impl Modal for AutoComplete {
    fn map(&mut self, key: &KeyEvent) {
        todo!();
    }
    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        let screen_part = Rect { x, y, width: x + 20, height: y + 5 };
        let complitions =
            self.completions.iter().map(|c| ListItem::new(c.label.as_str())).collect::<Vec<ListItem<'_>>>();
        let list = List::new(complitions).block(Block::default().borders(Borders::all()));
        frame.render_widget(Clear, screen_part);
        frame.render_widget(list, screen_part);
    }
}

impl AutoComplete {
    pub fn new(c: &CursorPosition, completions: Vec<CompletionItem>) -> Self {
        Self { cursor: *c, completions }
    }
}
