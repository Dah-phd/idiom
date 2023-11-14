mod parser;
use crate::{configs::EditorAction, events::WorkspaceEvent};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::{CompletionItem, Documentation, Hover, HoverContents, MarkedString, SignatureHelp};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{
    backend::CrosstermBackend,
    prelude::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::io::Stdout;

#[derive(Default)]
pub enum LSPModal {
    AutoComplete(AutoComplete),
    Info(Info),
    #[default]
    None,
}

#[derive(Default)]
pub enum LSPModalResult {
    Teken,
    #[default]
    None,
    Done,
    TakenDone,
    Workspace(WorkspaceEvent),
}

impl LSPModal {
    pub fn map_and_finish(&mut self, key: &EditorAction) -> LSPModalResult {
        match self {
            Self::None => LSPModalResult::None,
            Self::AutoComplete(modal) => modal.map_and_finish(key),
            Self::Info(..) => LSPModalResult::Done,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::None;
    }

    pub fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        if let Some(area) = derive_model_rect(x, y + 1, frame.size()) {
            match self {
                Self::None => (),
                Self::AutoComplete(modal) => {
                    modal.render_at(frame, area);
                }
                Self::Info(modal) => {
                    modal.render_at(frame, area);
                }
            }
        }
    }

    pub fn auto_complete(&mut self, completions: Vec<CompletionItem>, line: String, idx: usize) {
        if let Some(modal) = AutoComplete::new(completions, line, idx) {
            *self = Self::AutoComplete(modal);
        }
    }

    pub fn hover(&mut self, hover: Hover) {
        *self = Self::Info(Info::from_hover(hover));
    }

    pub fn signature(&mut self, signature: SignatureHelp) {
        *self = Self::Info(Info::from_signature(signature));
    }
}

pub struct AutoComplete {
    state: ListState,
    filter: String,
    line: String,
    matcher: SkimMatcherV2,
    filtered: Vec<(String, i64)>,
    completions: Vec<CompletionItem>,
}

impl AutoComplete {
    fn new(completions: Vec<CompletionItem>, line: String, idx: usize) -> Option<Self> {
        let mut filter = String::new();
        for ch in line[..idx].chars() {
            if ch.is_alphabetic() || ch == '_' {
                filter.push(ch);
            } else {
                filter.clear();
            }
        }
        let mut modal = Self {
            state: ListState::default(),
            line,
            filter,
            filtered: Vec::new(),
            completions,
            matcher: SkimMatcherV2::default(),
        };
        modal.build_matches();
        if modal.filter.is_empty() {
            None
        } else {
            Some(modal)
        }
    }

    fn map_and_finish(&mut self, key: &EditorAction) -> LSPModalResult {
        if self.completions.is_empty() || self.filtered.is_empty() {
            return LSPModalResult::Done;
        }
        match key {
            EditorAction::Close => LSPModalResult::TakenDone,
            EditorAction::NewLine | EditorAction::Indent => self.get_result(),
            EditorAction::Char(ch) => self.push_filter(*ch),
            EditorAction::Down => self.down(),
            EditorAction::Up => self.up(),
            EditorAction::Backspace => self.filter_pop(),
            _ => LSPModalResult::Done,
        }
    }

    fn get_result(&mut self) -> LSPModalResult {
        if let Some(idx) = self.state.selected() {
            LSPModalResult::Workspace(WorkspaceEvent::AutoComplete(self.filtered.remove(idx).0))
        } else {
            LSPModalResult::Done
        }
    }

    fn filter_pop(&mut self) -> LSPModalResult {
        self.filter.pop();
        if self.filter.is_empty() {
            LSPModalResult::Done
        } else {
            LSPModalResult::default()
        }
    }

    fn push_filter(&mut self, ch: char) -> LSPModalResult {
        if ch.is_alphabetic() || ch == '_' {
            self.filter.push(ch);
            self.build_matches();
            LSPModalResult::default()
        } else {
            LSPModalResult::Done
        }
    }

    fn build_matches(&mut self) {
        self.filtered = self
            .completions
            .iter()
            .filter_map(|item| {
                self.matcher.fuzzy_match(&item.label, &self.filter).map(|score| (item.label.to_owned(), score))
            })
            .collect();
        self.filtered.sort_by(|(_, idx), (_, rhidx)| rhidx.cmp(idx));
        self.state.select(Some(0));
    }

    fn down(&mut self) -> LSPModalResult {
        if let Some(idx) = self.state.selected() {
            let new_idx = idx + 1;
            self.state.select(Some(if self.filtered.len() > new_idx { new_idx } else { 0 }));
        }
        LSPModalResult::Teken
    }

    fn up(&mut self) -> LSPModalResult {
        if let Some(idx) = self.state.selected() {
            self.state.select(Some(idx.checked_sub(1).unwrap_or(self.filtered.len() - 1)));
        }
        LSPModalResult::Teken
    }

    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, area: Rect) {
        let complitions =
            self.filtered.iter().map(|(item, _)| ListItem::new(item.as_str())).collect::<Vec<ListItem<'_>>>();
        frame.render_widget(Clear, area);
        frame.render_stateful_widget(
            List::new(complitions)
                .block(Block::default().borders(Borders::all()))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
            &mut self.state,
        );
    }
}

pub struct Info {
    items: Vec<ListItem<'static>>,
}

impl Info {
    fn render_at(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, area: Rect) {
        frame.render_widget(Clear, area);
        frame.render_widget(List::new(self.items.as_slice()).block(Block::default().borders(Borders::all())), area);
    }

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

pub fn should_complete(line: &str, idx: usize) -> bool {
    let mut last_char = ' ';
    for (char_idx, ch) in line.char_indices() {
        if char_idx + 1 == idx && (last_char.is_whitespace() || last_char == '(') && (ch.is_alphabetic() || ch == '_') {
            return true;
        }
        last_char = ch;
    }
    false
}

fn derive_model_rect(mut x: u16, mut y: u16, base: Rect) -> Option<Rect> {
    let mut width = 60; //planned -> min 30 chars
    let mut height = 7; //planned -> min 5 with 3 completions
    if base.height < height + y {
        if base.height < 5 + y {
            if let Some(new_y) = y.checked_sub(8) {
                if y > base.height {
                    return None; // ! frame work sometime produces wierd y > protections against it
                }
                y = new_y;
            } else {
                y = y.checked_sub(6)?;
                height = 5;
            }
        } else {
            height = 5;
        }
    };
    if base.width < width + x {
        if base.width < 30 + x {
            x = base.width.checked_sub(30)?;
            width = 30;
        } else {
            width = base.width - x;
        }
    };
    Some(Rect { x, y, width, height })
}
