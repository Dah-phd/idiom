mod parser;

use crate::{
    configs::EditorAction,
    global_state::WorkspaceEvent,
    widgests::{dynamic_cursor_rect_sized_height, WrappedState},
    workspace::CursorPosition,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::{
    CompletionItem, Documentation, Hover, HoverContents, MarkedString, SignatureHelp, SignatureInformation,
};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{
    prelude::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
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
    Workspace(WorkspaceEvent),
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
    pub fn map_and_finish(&mut self, action: &EditorAction) -> ModalMessage {
        if matches!(action, EditorAction::Cancel | EditorAction::Close) {
            return ModalMessage::TakenDone;
        }
        match self {
            Self::AutoComplete(modal) => modal.map_and_finish(action),
            Self::Info(modal) => modal.map_and_finish(action),
            Self::RenameVar(modal) => modal.map_and_finish(action),
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
    new_name: String,
    cursor: CursorPosition,
    title: String,
}

impl RenameVariable {
    fn new(cursor: CursorPosition, title: &str) -> Self {
        Self { new_name: String::new(), cursor, title: format!("Rename: {} ", title) }
    }

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default().title(self.title.as_str()).borders(Borders::ALL);
        let p = Paragraph::new(Line::from(vec![
            Span::raw(" >> "),
            Span::raw(self.new_name.as_str()),
            Span::styled("|", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]));
        frame.render_widget(p.block(block), area);
    }

    fn map_and_finish(&mut self, action: &EditorAction) -> ModalMessage {
        match action {
            EditorAction::Char(ch) => {
                self.new_name.push(*ch);
                ModalMessage::Taken
            }
            EditorAction::Backspace => {
                self.new_name.pop();
                ModalMessage::Taken
            }
            EditorAction::NewLine => ModalMessage::RenameVar(self.new_name.to_owned(), self.cursor),
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
            // line,
            filter,
            filtered: Vec::new(),
            completions,
            matcher: SkimMatcherV2::default(),
        };
        modal.build_matches();
        modal
    }

    fn map_and_finish(&mut self, action: &EditorAction) -> ModalMessage {
        match action {
            EditorAction::NewLine | EditorAction::Indent => self.get_result(),
            EditorAction::Char(ch) => self.push_filter(*ch),
            EditorAction::Down => {
                self.state.next(&self.filtered);
                ModalMessage::Taken
            }
            EditorAction::Up => {
                self.state.prev(&self.filtered);
                ModalMessage::Taken
            }
            EditorAction::Backspace => self.filter_pop(),
            _ => ModalMessage::Done,
        }
    }

    fn get_result(&mut self) -> ModalMessage {
        if let Some(idx) = self.state.selected() {
            ModalMessage::Workspace(WorkspaceEvent::AutoComplete(self.filtered.remove(idx).0))
        } else {
            ModalMessage::Done
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
        let complitions =
            self.filtered.iter().map(|(item, _)| ListItem::new(item.as_str())).collect::<Vec<ListItem<'_>>>();
        frame.render_stateful_widget(
            List::new(complitions)
                .block(Block::default().borders(Borders::all()))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
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

    pub fn map_and_finish(&mut self, action: &EditorAction) -> ModalMessage {
        match action {
            EditorAction::Down => {
                self.state.next(&self.items);
                ModalMessage::Taken
            }
            EditorAction::Up => {
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
