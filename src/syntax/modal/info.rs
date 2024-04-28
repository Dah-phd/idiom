use super::ModalMessage;
use crate::syntax::{
    line_builder::{Action, DiagnosticInfo},
    LineBuilder, LineBuilderContext,
};
use crate::{
    global_state::GlobalState,
    utils::{BORDERED_BLOCK, REVERSED},
};
use crossterm::event::{KeyCode, KeyEvent};
use lsp_types::{Documentation, Hover, HoverContents, MarkedString, SignatureHelp, SignatureInformation};
use ratatui::prelude::Text;
use ratatui::style::Stylize;
use ratatui::text::Span;
use ratatui::widgets::Wrap;
use ratatui::{
    prelude::Rect,
    text::Line,
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};
use std::cmp::Ordering;

#[derive(Default)]
enum Mode {
    #[default]
    Full,
    Select,
}

#[derive(Default)]
pub struct Info {
    actions: Option<Vec<Action>>,
    text: Text<'static>,
    state: ListState,
    mode: Mode,
    at_line: u16,
}

impl Info {
    pub fn from_info(mut info: DiagnosticInfo) -> Self {
        let actions = info.actions.take();
        let mut lines = Vec::new();
        for span in info.messages.into_iter() {
            let s = span.style;
            for line in span.content.trim_start().lines() {
                lines.push(Line::from(line.to_owned()).style(s));
            }
        }
        let mode = if actions.is_some() { Mode::Select } else { Mode::Full };
        Self { actions, text: Text::from(lines), mode, ..Default::default() }
    }

    pub fn from_hover(hover: Hover, line_builder: &LineBuilder) -> Self {
        let mut lines = Vec::new();
        parse_hover(hover, line_builder, &mut lines);
        Self { text: Text::from(lines), ..Default::default() }
    }

    pub fn from_signature(signature: SignatureHelp, line_builder: &LineBuilder) -> Self {
        let mut lines = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, line_builder, &mut lines);
        }
        Self { text: Text::from(lines), ..Default::default() }
    }

    pub fn len(&self) -> usize {
        match self.mode {
            Mode::Full => self.text.lines.len(),
            Mode::Select => self.actions.as_ref().map(|i| i.len()).unwrap_or_default() + self.text.lines.len(),
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        if self.text.lines.is_empty() && self.actions.is_none() {
            return ModalMessage::Done;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Right => {
                if !matches!(self.mode, Mode::Select) {
                    return ModalMessage::Done;
                }
                if let Some(mut i) = self.actions.take() {
                    if let Some(idx) = self.state.selected() {
                        return match i.len().cmp(&idx) {
                            Ordering::Greater => {
                                gs.workspace.push(i.remove(idx).into());
                                ModalMessage::TakenDone
                            }
                            _ => {
                                self.mode = Mode::Full;
                                ModalMessage::Taken
                            }
                        };
                    }
                }
                ModalMessage::Done
            }
            KeyCode::Up => self.prev(),
            KeyCode::Down => self.next(),
            KeyCode::Left if !matches!(self.mode, Mode::Select) && self.actions.is_some() => {
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
            _ => {
                self.at_line = self.at_line.saturating_add(1);
            }
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
            _ => {
                self.at_line = self.at_line.saturating_sub(1);
            }
        }

        ModalMessage::Taken
    }

    pub fn push_hover(&mut self, hover: Hover, line_builder: &LineBuilder) {
        parse_hover(hover, line_builder, &mut self.text.lines);
        self.state.select(None);
    }

    pub fn push_signature(&mut self, signature: SignatureHelp, line_builder: &LineBuilder) {
        for info in signature.signatures {
            parse_sig_info(info, line_builder, &mut self.text.lines);
        }
        self.state.select(None);
    }

    pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
        match self.mode {
            Mode::Select => {
                if let Some(actions) = self.actions.as_ref() {
                    let mut list = actions.iter().map(|a| a.to_string().into()).collect::<Vec<ListItem<'static>>>();
                    if !self.text.lines.is_empty() {
                        list.push("Information".into());
                    }
                    frame.render_stateful_widget(
                        List::new(list).block(BORDERED_BLOCK).highlight_style(REVERSED),
                        area,
                        &mut self.state,
                    );
                }
            }
            Mode::Full => {
                frame.render_widget(
                    Paragraph::new(self.text.clone())
                        .block(BORDERED_BLOCK)
                        .wrap(Wrap::default())
                        .scroll((self.at_line, 0)),
                    area,
                );
            }
        }
    }
}

impl From<DiagnosticInfo> for Info {
    fn from(actions: DiagnosticInfo) -> Self {
        Self::from_info(actions)
    }
}

fn parse_sig_info(info: SignatureInformation, builder: &LineBuilder, lines: &mut Vec<Line<'static>>) {
    let mut ctx = LineBuilderContext::default();
    // lines.push(Line::from(generic_line(builder, usize::MAX, &info.label, &mut ctx, Vec::new())));
    if let Some(text) = info.documentation {
        match text {
            Documentation::MarkupContent(c) => {
                if matches!(c.kind, lsp_types::MarkupKind::Markdown) {
                    let mut is_code = false;
                    for line in c.value.lines() {
                        if line.starts_with("```") {
                            is_code = !is_code;
                            continue;
                        }
                        if is_code {
                            // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                        } else {
                            lines.push(Line::from(String::from(line)));
                        }
                    }
                } else {
                    for line in c.value.lines() {
                        lines.push(Line::from(String::from(line)));
                    }
                }
            }
            Documentation::String(s) => {
                for line in s.lines() {
                    // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                }
            }
        }
    }
}

fn parse_hover(hover: Hover, builder: &LineBuilder, lines: &mut Vec<Line<'static>>) {
    match hover.contents {
        HoverContents::Array(arr) => {
            let mut ctx = LineBuilderContext::default();
            for value in arr {
                for line in parse_markedstr(value).lines() {
                    // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                }
            }
        }
        HoverContents::Markup(markup) => {
            handle_markup(markup, builder, lines);
        }
        HoverContents::Scalar(value) => {
            let mut ctx = LineBuilderContext::default();
            for line in parse_markedstr(value).lines() {
                // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
            }
        }
    }
}

fn handle_markup(markup: lsp_types::MarkupContent, builder: &LineBuilder, lines: &mut Vec<Line<'static>>) {
    let mut ctx = LineBuilderContext::default();
    if !matches!(markup.kind, lsp_types::MarkupKind::Markdown) {
        for line in markup.value.lines() {
            // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
        }
        return;
    }
    let mut is_code = false;
    for line in markup.value.lines() {
        if line.trim().starts_with("```") {
            is_code = !is_code;
            continue;
        }
        if is_code {
            // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
        } else if line.trim().starts_with('#') {
            lines.push(Line::from(Span::raw(line.to_owned()).bold()));
        } else {
            lines.push(Line::from(String::from(line)));
        }
    }
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
