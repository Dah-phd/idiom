mod parser;

use crate::{
    global_state::{GlobalState, WorkspaceEvent},
    utils::REVERSED,
    widgests::{dynamic_cursor_rect_sized_height, TextField, WrappedState},
    workspace::CursorPosition,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::{
    CompletionItem, Documentation, Hover, HoverContents, MarkedString, SignatureHelp, SignatureInformation,
};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{
    prelude::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub enum LSPModal {
    AutoComplete(AutoComplete),
    RenameVar(RenameVariable),
    Info(Info),
}

#[derive(Default, Debug)]
pub enum ModalMessage {
    Taken,
    #[default]
    None,
    Done,
    TakenDone,
    RenameVar(String, CursorPosition),
}

impl<T> From<&[T]> for ModalMessage {
    fn from(value: &[T]) -> Self {
        if value.is_empty() {
            ModalMessage::Done
        } else {
            ModalMessage::default()
        }
    }
}

impl LSPModal {
    pub fn map_and_finish(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        match key {
            KeyEvent { code: KeyCode::Esc, .. } => ModalMessage::TakenDone,
            KeyEvent { code: KeyCode::Char('q' | 'Q'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, .. } => {
                ModalMessage::TakenDone
            }
            _ => match self {
                Self::AutoComplete(modal) => modal.map_and_finish(key, gs),
                Self::Info(modal) => modal.map_and_finish(key),
                Self::RenameVar(modal) => modal.map_and_finish(key, gs),
            },
        }
    }

    pub fn render_at(&mut self, frame: &mut Frame, x: u16, y: u16) {
        match self {
            Self::AutoComplete(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(modal.filtered.len(), x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
            Self::RenameVar(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(1, x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
            Self::Info(modal) => {
                if let Some(area) = dynamic_cursor_rect_sized_height(modal.items.len(), x, y + 1, frame.size()) {
                    frame.render_widget(Clear, area);
                    modal.render_at(frame, area);
                }
            }
        }
    }

    pub fn auto_complete(completions: Vec<CompletionItem>, line: String, idx: usize) -> Option<Self> {
        let modal = AutoComplete::new(completions, line, idx);
        if !modal.filtered.is_empty() {
            return Some(LSPModal::AutoComplete(modal));
        }
        None
    }

    pub fn hover_map(&mut self, hover: Hover) {
        if let Self::Info(modal) = self {
            modal.push_hover(hover);
        } else {
            *self = Self::hover(hover);
        }
    }

    pub fn hover(hover: Hover) -> Self {
        Self::Info(Info::from_hover(hover))
    }

    pub fn signature_map(&mut self, signature: SignatureHelp) {
        if let Self::Info(modal) = self {
            modal.insert_signature(signature);
        } else {
            *self = Self::signature(signature);
        }
    }

    pub fn signature(signature: SignatureHelp) -> Self {
        Self::Info(Info::from_signature(signature))
    }

    pub fn renames_at(c: CursorPosition, title: &str) -> Self {
        Self::RenameVar(RenameVariable::new(c, title))
    }
}

pub struct RenameVariable {
    new_name: TextField<()>,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    fn new(cursor: CursorPosition, title: &str) -> Self {
        Self { new_name: TextField::basic(title.to_owned()), cursor, title: format!("Rename: {} ", title) }
    }

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default().title(self.title.as_str()).borders(Borders::ALL);
        frame.render_widget(self.new_name.widget().block(block), area);
    }

    fn map_and_finish(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        self.new_name.map(key, &mut gs.clipboard);
        match key.code {
            KeyCode::Enter => ModalMessage::RenameVar(self.new_name.text.to_owned(), self.cursor),
            _ => ModalMessage::Taken,
        }
    }
}

pub struct AutoComplete {
    state: WrappedState,
    filter: String,
    // line: String,
    matcher: SkimMatcherV2,
    filtered: Vec<(String, i64)>,
    active_doc: Option<Paragraph<'static>>,
    completions: Vec<CompletionItem>,
}

impl AutoComplete {
    fn new(completions: Vec<CompletionItem>, line: String, idx: usize) -> Self {
        let mut filter = String::new();
        for ch in line[..idx].chars() {
            if ch.is_alphabetic() || ch == '_' {
                filter.push(ch);
            } else {
                filter.clear();
            }
        }
        let mut modal = Self {
            state: WrappedState::default(),
            filter,
            // line,
            matcher: SkimMatcherV2::default(),
            filtered: Vec::new(),
            active_doc: None,
            completions,
        };
        modal.build_matches();
        modal
    }

    fn map_and_finish(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        match key.code {
            KeyCode::F(1) => {
                self.active_doc = self.get_docs();
                ModalMessage::Taken
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(idx) = self.state.selected() {
                    gs.workspace.push_back(WorkspaceEvent::AutoComplete(self.filtered.remove(idx).0));
                }
                ModalMessage::TakenDone
            }
            KeyCode::Char(ch) => self.push_filter(ch),
            KeyCode::Down => {
                self.active_doc = None;
                self.state.next(&self.filtered);
                ModalMessage::Taken
            }
            KeyCode::Up => {
                self.active_doc = None;
                self.state.prev(&self.filtered);
                ModalMessage::Taken
            }
            KeyCode::Backspace => self.filter_pop(),
            _ => ModalMessage::Done,
        }
    }

    fn filter_pop(&mut self) -> ModalMessage {
        self.filter.pop();
        self.build_matches();
        if self.filter.is_empty() {
            return ModalMessage::Done;
        }
        self.filtered.as_slice().into()
    }

    fn push_filter(&mut self, ch: char) -> ModalMessage {
        if ch.is_alphabetic() || ch == '_' {
            self.filter.push(ch);
            self.build_matches();
            self.filtered.as_slice().into()
        } else {
            ModalMessage::Done
        }
    }

    fn get_docs(&mut self) -> Option<Paragraph<'static>> {
        let label = &self.filtered.get(self.state.selected()?)?.0;
        self.completions.iter().find(|item| &item.label == label).and_then(|item| {
            item.documentation.as_ref().map(|docs| match docs {
                Documentation::String(value) => Paragraph::new(value.to_owned()),
                Documentation::MarkupContent(content) => Paragraph::new(content.value.to_owned()),
            })
        })
    }

    fn build_matches(&mut self) {
        self.filtered = self
            .completions
            .iter()
            .filter_map(|item| {
                self.matcher.fuzzy_match(&item.label, &self.filter).map(|score| {
                    let divisor = item.label.len().abs_diff(self.filter.len()) as i64;
                    let new_score = if divisor != 0 { score / divisor } else { score };
                    (item.label.to_owned(), new_score)
                })
            })
            .collect();
        self.filtered.sort_by(|(_, idx), (_, rhidx)| rhidx.cmp(idx));
        self.state.set(0);
    }

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::all());
        if let Some(docs) = self.active_doc.as_ref() {
            frame.render_widget(docs.clone().block(block), area);
            return;
        }
        let complitions = self.filtered.iter().map(|(item, _)| ListItem::new(item.as_str())).collect::<Vec<_>>();
        frame.render_stateful_widget(
            List::new(complitions).block(block).highlight_style(REVERSED),
            area,
            self.state.get(),
        );
    }
}

pub struct Info {
    items: Vec<ListItem<'static>>,
    state: WrappedState,
}

impl Info {
    pub fn from_hover(hover: Hover) -> Self {
        let mut items = Vec::new();
        parse_hover(hover, &mut items);
        Self { items, state: WrappedState::default() }
    }

    pub fn from_signature(signature: SignatureHelp) -> Self {
        let mut items = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, &mut items);
        }
        Self { items, state: WrappedState::default() }
    }

    pub fn map_and_finish(&mut self, key: &KeyEvent) -> ModalMessage {
        match key.code {
            KeyCode::Down => {
                self.state.next(&self.items);
                ModalMessage::Taken
            }
            KeyCode::Up => {
                self.state.prev(&self.items);
                ModalMessage::Taken
            }
            _ => ModalMessage::Done,
        }
    }

    pub fn push_hover(&mut self, hover: Hover) {
        parse_hover(hover, &mut self.items);
        self.state.drop();
    }

    pub fn insert_signature(&mut self, signature: SignatureHelp) {
        for info in signature.signatures {
            parse_sig_info(info, &mut self.items);
        }
        self.state.drop();
    }

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            List::new(self.items.clone()).block(Block::default().borders(Borders::all())),
            area,
            self.state.get(),
        );
    }
}

fn parse_sig_info(info: SignatureInformation, items: &mut Vec<ListItem<'static>>) {
    let mut idx = 0;
    items.insert(idx, ListItem::new(Line::from(info.label)));
    if let Some(text) = info.documentation {
        match text {
            Documentation::MarkupContent(c) => {
                for line in c.value.lines() {
                    idx += 1;
                    items.insert(idx, ListItem::new(Line::from(String::from(line))));
                }
            }
            Documentation::String(s) => {
                for line in s.lines() {
                    idx += 1;
                    items.insert(idx, ListItem::new(Line::from(String::from(line))));
                }
            }
        }
    }
}

fn parse_hover(hover: Hover, items: &mut Vec<ListItem<'static>>) {
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
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
