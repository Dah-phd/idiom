mod parser;
use std::fmt::format;

use crate::{components::workspace::CursorPosition, configs::EditorAction, events::WorkspaceEvent};
use fuzzy_matcher::{
    skim::{SkimMatcherV2, SkimScoreConfig},
    FuzzyMatcher,
};
use lsp_types::{CompletionItem, Documentation, Hover, HoverContents, MarkedString, SignatureHelp};
pub use parser::{LSPResponseType, LSPResult};
use ratatui::{
    prelude::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
pub enum LSPModal {
    AutoComplete(AutoComplete),
    RenameVar(RenameVariable),
    Info(Info),
}

#[derive(Default, Debug)]
pub enum LSPModalResult {
    Taken,
    #[default]
    None,
    Done,
    TakenDone,
    Workspace(WorkspaceEvent),
    RenameVar(String, CursorPosition),
}

impl<T> From<&[T]> for LSPModalResult {
    fn from(value: &[T]) -> Self {
        if value.is_empty() {
            LSPModalResult::Done
        } else {
            LSPModalResult::default()
        }
    }
}

impl LSPModal {
    pub fn map_and_finish(&mut self, action: &EditorAction) -> LSPModalResult {
        if matches!(action, EditorAction::Cancel | EditorAction::Close) {
            return LSPModalResult::TakenDone;
        }
        match self {
            Self::AutoComplete(modal) => modal.map_and_finish(action),
            Self::Info(..) => LSPModalResult::Done,
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

    pub fn hover(hover: Hover) -> Self {
        Self::Info(Info::from_hover(hover))
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

    fn map_and_finish(&mut self, action: &EditorAction) -> LSPModalResult {
        match action {
            EditorAction::Char(ch) => {
                self.new_name.push(*ch);
                LSPModalResult::Taken
            }
            EditorAction::Backspace => {
                self.new_name.pop();
                LSPModalResult::Taken
            }
            EditorAction::NewLine => LSPModalResult::RenameVar(self.new_name.to_owned(), self.cursor),
            _ => LSPModalResult::Taken,
        }
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
            state: ListState::default(),
            line,
            filter,
            filtered: Vec::new(),
            completions,
            matcher: SkimMatcherV2::default().score_config(SkimScoreConfig { score_match: 1000, ..Default::default() }),
        };
        modal.build_matches();
        modal
    }

    fn map_and_finish(&mut self, action: &EditorAction) -> LSPModalResult {
        match action {
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
        self.build_matches();
        if self.filter.is_empty() {
            return LSPModalResult::Done;
        }
        self.filtered.as_slice().into()
    }

    fn push_filter(&mut self, ch: char) -> LSPModalResult {
        if ch.is_alphabetic() || ch == '_' {
            self.filter.push(ch);
            self.build_matches();
            self.filtered.as_slice().into()
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
        LSPModalResult::Taken
    }

    fn up(&mut self) -> LSPModalResult {
        if let Some(idx) = self.state.selected() {
            self.state.select(Some(idx.checked_sub(1).unwrap_or(self.filtered.len() - 1)));
        }
        LSPModalResult::Taken
    }

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        let complitions =
            self.filtered.iter().map(|(item, _)| ListItem::new(item.as_str())).collect::<Vec<ListItem<'_>>>();
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

    fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(List::new(self.items.as_slice()).block(Block::default().borders(Borders::all())), area);
    }
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}

fn dynamic_cursor_rect_sized_height(
    lines: usize, // min 3
    mut x: u16,
    mut y: u16,
    base: Rect,
) -> Option<Rect> {
    //  ______________
    // |y,x _____     |
    // |   |     |    | base hight (y)
    // |   |     | h..|
    // |   |     |    |
    // |    -----     |
    // |    width(60) |
    //  --------------
    //   base.width (x)
    //
    let mut height = (lines.min(5) + 2) as u16;
    let mut width = 60;
    if base.height < height + y {
        if base.height > 3 + y {
            height = base.height - y;
        } else if y > 3 && base.height > y {
            // ensures overflowed y's are handled
            let new_y = y.saturating_sub(height + 1);
            height = y - (new_y + 1);
            y = new_y;
        } else {
            return None;
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
