use std::cmp::Ordering;

use crate::{
    global_state::{GlobalState, WorkspaceEvent},
    utils::{BORDERED_BLOCK, REVERSED},
    workspace::CursorPosition,
};
use crossterm::event::{KeyCode, KeyEvent};
use lsp_types::{Documentation, Hover, HoverContents, MarkedString, SignatureHelp, SignatureInformation};
use ratatui::{
    prelude::Rect,
    text::Line,
    widgets::{List, ListItem, ListState},
    Frame,
};

use super::ModalMessage;

#[derive(Default)]
enum Mode {
    #[default]
    Full,
    Select,
    Hover,
    Signature,
}

#[derive(Default)]
pub struct Info {
    imports: Option<Vec<String>>,
    hover: Option<Vec<ListItem<'static>>>,
    signitures: Option<Vec<ListItem<'static>>>,
    state: ListState,
    mode: Mode,
    at_line: usize,
}

impl Info {
    pub fn from_imports(imports: Vec<String>) -> Self {
        Self { imports: Some(imports), mode: Mode::Select, ..Default::default() }
    }

    pub fn from_hover(hover: Hover) -> Self {
        Self { hover: Some(parse_hover(hover)), ..Default::default() }
    }

    pub fn from_signature(signature: SignatureHelp) -> Self {
        let mut items = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, &mut items);
        }
        Self { signitures: Some(items), ..Default::default() }
    }

    pub fn len(&self) -> usize {
        match self.mode {
            Mode::Full => {
                self.hover.as_ref().map(|h| h.len()).unwrap_or_default()
                    + self.signitures.as_ref().map(|s| s.len()).unwrap_or_default()
            }
            Mode::Select => {
                let mut len = self.imports.as_ref().map(|i| i.len()).unwrap_or_default();
                if self.hover.is_some() {
                    len += 1;
                }
                if self.signitures.is_some() {
                    len += 1;
                }
                len
            }
            Mode::Hover => self.hover.as_ref().map(|h| h.len()).unwrap_or_default(),
            Mode::Signature => self.signitures.as_ref().map(|s| s.len()).unwrap_or_default(),
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        if self.hover.is_none() && self.signitures.is_none() && self.imports.is_none() {
            return ModalMessage::Done;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Right => {
                if !matches!(self.mode, Mode::Select) {
                    return ModalMessage::Done;
                }
                if let Some(mut i) = self.imports.take() {
                    if let Some(idx) = self.state.selected() {
                        return match i.len().cmp(&idx) {
                            Ordering::Greater => {
                                gs.workspace.push_back(WorkspaceEvent::ReplaceNextSelect {
                                    new_text: i.remove(idx),
                                    select: (CursorPosition::default(), CursorPosition::default()),
                                    next_select: None,
                                });
                                ModalMessage::TakenDone
                            }
                            Ordering::Equal if self.hover.is_some() => {
                                self.mode = Mode::Hover;
                                ModalMessage::Taken
                            }
                            _ => {
                                self.mode = Mode::Signature;
                                ModalMessage::Taken
                            }
                        };
                    }
                }
                ModalMessage::Done
            }
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Left if !matches!(self.mode, Mode::Select) && self.imports.is_some() => {
                self.mode = Mode::Select;
                ModalMessage::Taken
            }
            _ => ModalMessage::Done,
        }
    }

    pub fn next(&mut self) -> ModalMessage {
        match self.mode {
            Mode::Select => {
                let len = self.len();
                match self.state.selected() {
                    Some(idx) => {
                        let idx = idx + 1;
                        self.state.select(Some(if idx < len { idx } else { 0 }));
                    }
                    None if len != 0 => self.state.select(Some(0)),
                    _ => (),
                }
            }
            _ => self.at_line = std::cmp::min(self.len() - 1, self.at_line + 1),
        }
        ModalMessage::Taken
    }

    pub fn prev(&mut self) -> ModalMessage {
        match self.mode {
            Mode::Select => {
                let len = self.len();
                match self.state.selected() {
                    Some(idx) => self.state.select(Some(if idx == 0 { len - 1 } else { idx - 1 })),
                    None if len > 0 => self.state.select(Some(len - 1)),
                    _ => (),
                }
            }
            _ => self.at_line = self.at_line.saturating_sub(1),
        }

        ModalMessage::Taken
    }

    pub fn push_hover(&mut self, hover: Hover) {
        self.hover = Some(parse_hover(hover));
        self.state.select(None);
    }

    pub fn push_signature(&mut self, signature: SignatureHelp) {
        let mut items = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, &mut items);
        }
        self.signitures = Some(items);
        self.state.select(None);
    }

    pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        match self.mode {
            Mode::Select => {
                if let Some(imports) = self.imports.as_ref() {
                    let mut list =
                        imports.iter().map(|i| format!("import: {i}").into()).collect::<Vec<ListItem<'static>>>();
                    if self.hover.is_some() {
                        list.push("Hover Info".into());
                    }
                    if self.signitures.is_some() {
                        list.push("Signiture Help".into());
                    }
                    frame.render_stateful_widget(
                        List::new(list).block(BORDERED_BLOCK).highlight_style(REVERSED),
                        area,
                        &mut self.state,
                    );
                }
            }
            Mode::Hover => {
                if let Some(list) = self.hover.as_ref().map(|h| h.iter().skip(self.at_line).cloned()) {
                    frame.render_widget(List::new(list).block(BORDERED_BLOCK), area);
                }
            }
            Mode::Signature => {
                if let Some(list) = self.signitures.as_ref().map(|h| h.iter().skip(self.at_line).cloned()) {
                    frame.render_widget(List::new(list).block(BORDERED_BLOCK), area);
                }
            }
            Mode::Full => {
                let mut list = Vec::new();
                let mut skip = 0;
                if let Some(hovers) = self.hover.as_ref() {
                    list.extend(hovers.iter().skip(self.at_line).cloned());
                    if list.is_empty() {
                        skip = self.at_line.saturating_sub(hovers.len());
                    }
                }
                if let Some(signitures) = self.signitures.as_ref() {
                    list.extend(signitures.iter().skip(skip).cloned());
                }
                frame.render_widget(List::new(list).block(BORDERED_BLOCK), area);
            }
        }
    }
}

impl From<Vec<String>> for Info {
    fn from(imports: Vec<String>) -> Self {
        Self::from_imports(imports)
    }
}

impl From<Hover> for Info {
    fn from(hover: Hover) -> Self {
        Self::from_hover(hover)
    }
}

impl From<SignatureHelp> for Info {
    fn from(signature: SignatureHelp) -> Self {
        Self::from_signature(signature)
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

fn parse_hover(hover: Hover) -> Vec<ListItem<'static>> {
    let mut buffer = Vec::new();
    match hover.contents {
        HoverContents::Array(arr) => {
            for value in arr {
                for line in parse_markedstr(value).lines() {
                    buffer.push(ListItem::new(Line::from(String::from(line))));
                }
            }
        }
        HoverContents::Markup(markup) => {
            for line in markup.value.lines() {
                buffer.push(ListItem::new(Line::from(String::from(line))));
            }
        }
        HoverContents::Scalar(value) => {
            for line in parse_markedstr(value).lines() {
                buffer.push(ListItem::new(Line::from(String::from(line))));
            }
        }
    }
    buffer
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
