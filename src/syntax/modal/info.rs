use super::ModalMessage;
use crate::{
    global_state::GlobalState,
    render::{
        backend::{color, Color, Style},
        layout::Rect,
        state::State,
        widgets::paragraph_styled,
    },
    syntax::{Action, DiagnosticInfo},
};
use crossterm::event::{KeyCode, KeyEvent};
use lsp_types::{Documentation, Hover, HoverContents, MarkedString, SignatureHelp, SignatureInformation};
use std::cmp::Ordering;

#[derive(Default)]
enum Mode {
    #[default]
    Text,
    Select,
}

#[derive(Default)]
pub struct Info {
    actions: Option<Vec<Action>>,
    text: Vec<(String, Color)>,
    state: State,
    text_state: usize,
    mode: Mode,
}

impl Info {
    pub fn from_info(info: DiagnosticInfo) -> Self {
        let mode = if info.actions.is_some() { Mode::Select } else { Mode::Text };
        let mut text = Vec::new();
        for (msg, color) in info.messages.into_iter() {
            for line in msg.lines() {
                text.push((String::from(line), color));
            }
        }
        Self { actions: info.actions, text, mode, ..Default::default() }
    }

    pub fn from_hover(hover: Hover) -> Self {
        let mut lines = Vec::new();
        parse_hover(hover, &mut lines);
        Self { text: lines, ..Default::default() }
    }

    pub fn from_signature(signature: SignatureHelp) -> Self {
        let mut lines = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, &mut lines);
        }
        Self { text: lines, ..Default::default() }
    }

    pub fn len(&self) -> usize {
        match self.mode {
            Mode::Text => self.text.len(),
            Mode::Select => self.actions.as_ref().map(|i| i.len()).unwrap_or_default() + self.text.len(),
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> ModalMessage {
        if self.text.is_empty() && self.actions.is_none() {
            return ModalMessage::Done;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Right => {
                if !matches!(self.mode, Mode::Select) {
                    return ModalMessage::Done;
                }
                if let Some(mut i) = self.actions.take() {
                    return match i.len().cmp(&self.state.selected) {
                        Ordering::Greater => {
                            gs.workspace.push(i.remove(self.state.selected).into());
                            ModalMessage::TakenDone
                        }
                        _ => {
                            self.mode = Mode::Text;
                            ModalMessage::Taken
                        }
                    };
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
                self.state.next(self.len());
            }
            Mode::Text => {
                if self.text_state < self.text.len() {
                    self.text_state += 1;
                }
            }
        }
        ModalMessage::Taken
    }

    pub fn prev(&mut self) -> ModalMessage {
        match self.mode {
            Mode::Select => {
                self.state.prev(self.len());
            }
            Mode::Text => {
                self.text_state = self.text_state.saturating_sub(1);
            }
        }

        ModalMessage::Taken
    }

    pub fn push_hover(&mut self, hover: Hover) {
        parse_hover(hover, &mut self.text);
        self.state.selected = 0;
    }

    pub fn push_signature(&mut self, signature: SignatureHelp) {
        for info in signature.signatures {
            parse_sig_info(info, &mut self.text);
        }
        self.state.selected = 0;
    }

    pub fn render(&mut self, rect: &Rect, gs: &mut GlobalState) {
        match self.mode {
            Mode::Select => {
                if let Some(actions) = self.actions.as_ref() {
                    let actions = actions.iter().map(|a| a.to_string()).collect::<Vec<_>>();
                    let options = actions.iter().map(|s| s.as_str());
                    if !self.text.is_empty() {
                        self.state.render_list(options.chain(["Information"]), rect, &mut gs.writer);
                    } else {
                        self.state.render_list(options, rect, &mut gs.writer);
                    };
                }
            }
            Mode::Text => {
                paragraph_styled(
                    *rect,
                    self.text.iter().skip(self.text_state).map(|(d, c)| (d.as_str(), Style::fg(*c))),
                    &mut gs.writer,
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

fn parse_sig_info(info: SignatureInformation, lines: &mut Vec<(String, Color)>) {
    lines.push((info.label, color::reset()));
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
                            lines.push((String::from(line), color::reset()));
                            // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                        } else {
                            lines.push((String::from(line), color::reset()));
                        }
                    }
                } else {
                    for line in c.value.lines() {
                        lines.push((String::from(line), color::reset()));
                    }
                }
            }
            Documentation::String(s) => {
                for line in s.lines() {
                    lines.push((String::from(line), color::reset()));
                    // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                }
            }
        }
    }
}

fn parse_hover(hover: Hover, lines: &mut Vec<(String, Color)>) {
    match hover.contents {
        HoverContents::Array(arr) => {
            // let mut ctx = LineBuilderContext::default();
            for value in arr {
                for line in parse_markedstr(value).lines() {
                    lines.push((String::from(line), color::reset()));
                    // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                }
            }
        }
        HoverContents::Markup(markup) => {
            handle_markup(markup, lines);
        }
        HoverContents::Scalar(value) => {
            for line in parse_markedstr(value).lines() {
                // TODO parse to tokens
                lines.push((line.to_owned(), color::reset()))
                // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
            }
        }
    }
}

fn handle_markup(markup: lsp_types::MarkupContent, lines: &mut Vec<(String, Color)>) {
    if !matches!(markup.kind, lsp_types::MarkupKind::Markdown) {
        for line in markup.value.lines() {
            lines.push((line.to_owned(), color::reset()));
            // TODO parse to tokens
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
            lines.push((line.to_owned(), color::reset()));
        } else {
            lines.push((line.to_owned(), color::reset()))
        }
    }
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
