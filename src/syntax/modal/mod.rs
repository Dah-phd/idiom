mod parser;
use crate::{components::workspace::CursorPosition, configs::EditorAction};
use lsp_types::{CompletionItem, Documentation, Hover, HoverContents, MarkedString, SignatureHelp};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};
use std::io::Stdout;

pub trait Modal {
    fn map_and_finish(&mut self, key: &EditorAction) -> bool;
    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16);
}

#[derive(Debug)]
pub struct AutoComplete {
    cursor: CursorPosition,
    completions: Vec<CompletionItem>,
}

impl Modal for AutoComplete {
    fn map_and_finish(&mut self, _key: &EditorAction) -> bool {
        true
    }

    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        let base = frame.size();
        let screen_part = Rect { x, y: y + 1, width: base.width.min(45), height: base.height.min(7) };
        let complitions =
            self.completions.iter().map(|c| ListItem::new(c.label.as_str())).collect::<Vec<ListItem<'_>>>();
        frame.render_widget(Clear, screen_part);
        frame.render_widget(List::new(complitions).block(Block::default().borders(Borders::all())), screen_part);
    }
}

impl AutoComplete {
    pub fn new(c: &CursorPosition, completions: Vec<CompletionItem>) -> Self {
        Self { cursor: *c, completions }
    }
}

pub struct Info<'a> {
    items: Vec<ListItem<'a>>,
}

impl<'a> Modal for Info<'a> {
    fn map_and_finish(&mut self, _key: &EditorAction) -> bool {
        true
    }

    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        let base = frame.size();
        let screen_part = Rect { x, y: y + 1, width: base.width.min(60), height: base.height.min(7) };
        frame.render_widget(Clear, screen_part);
        frame.render_widget(
            List::new(self.items.as_slice()).block(Block::default().borders(Borders::all())),
            screen_part,
        );
    }
}

impl<'a> Info<'a> {
    pub fn from_hover(hover: Hover) -> Self {
        let mut items = Vec::new();
        match hover.contents {
            HoverContents::Array(arr) => {
                for value in arr {
                    for line in parse_markedstr(value).lines() {
                        items.push(ListItem::new(Line::from(String::from(line))));
                    }
                }
            }
            HoverContents::Markup(markup) => {
                for line in markup.value.lines() {
                    items.push(ListItem::new(Line::from(String::from(line))));
                }
            }
            HoverContents::Scalar(value) => {
                for line in parse_markedstr(value).lines() {
                    items.push(ListItem::new(Line::from(String::from(line))));
                }
            }
        }
        Self { items }
    }

    pub fn from_signature(signature: SignatureHelp) -> Self {
        let mut items = Vec::new();
        for sig_help in signature.signatures {
            items.push(ListItem::new(Line::from(sig_help.label)));
            if let Some(text) = sig_help.documentation {
                match text {
                    Documentation::MarkupContent(c) => {
                        for line in c.value.lines() {
                            items.push(ListItem::new(Line::from(String::from(line))));
                        }
                    }
                    Documentation::String(s) => {
                        for line in s.lines() {
                            items.push(ListItem::new(Line::from(String::from(line))));
                        }
                    }
                }
            }
        }
        Self { items }
    }
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
