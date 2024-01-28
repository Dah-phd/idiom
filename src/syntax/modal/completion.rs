use super::ModalMessage;
use crate::{
    global_state::{GlobalState, WorkspaceEvent},
    utils::{BORDERED_BLOCK, REVERSED},
    widgests::WrappedState,
};
use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use lsp_types::{CompletionItem, Documentation};
use ratatui::{
    prelude::Rect,
    widgets::{List, ListItem, Paragraph},
    Frame,
};

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
    pub fn new(completions: Vec<CompletionItem>, line: String, idx: usize) -> Self {
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

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        match key.code {
            KeyCode::F(1) => {
                self.active_doc = self.get_docs();
                ModalMessage::Taken
            }
            KeyCode::Enter | KeyCode::Tab => {
                if let Some(idx) = self.state.selected() {
                    gs.workspace.push(WorkspaceEvent::AutoComplete(self.filtered.remove(idx).0));
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

    pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(docs) = self.active_doc.as_ref() {
            frame.render_widget(docs.clone().block(BORDERED_BLOCK), area);
            return;
        }
        let complitions = self.filtered.iter().map(|(item, _)| ListItem::new(item.as_str())).collect::<Vec<_>>();
        frame.render_stateful_widget(
            List::new(complitions).block(BORDERED_BLOCK).highlight_style(REVERSED),
            area,
            self.state.get(),
        );
    }

    pub fn len(&self) -> usize {
        self.filtered.len()
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
                self.matcher.fuzzy_match(item.filter_text.as_ref().unwrap_or(&item.label), &self.filter).map(|score| {
                    let divisor = item.label.len().abs_diff(self.filter.len()) as i64;
                    let new_score = if divisor != 0 { score / divisor } else { score };
                    (item.label.to_owned(), new_score)
                })
            })
            .collect();
        self.filtered.sort_by(|(_, idx), (_, rhidx)| rhidx.cmp(idx));
        self.state.set(0);
    }
}
