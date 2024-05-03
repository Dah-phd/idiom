use super::ModalMessage;
use crate::{
    global_state::GlobalState,
    render::{layout::Rect, state::State},
    syntax::{Action, DiagnosticInfo},
};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{Color, ContentStyle};
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
    text_state: State,
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
                self.text_state.next(self.len());
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
                self.text_state.prev(self.len());
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

    pub fn render(&mut self, area: &Rect, gs: &mut GlobalState) -> std::io::Result<()> {
        match self.mode {
            Mode::Select => {
                if let Some(actions) = self.actions.as_ref() {
                    let mut options = actions.iter().map(|a| a.to_string()).collect::<Vec<_>>();
                    if !self.text.is_empty() {
                        options.push("Information".into());
                    };
                    self.state.render_strings(&options, area, &mut gs.writer)?;
                }
            }
            Mode::Text => {
                self.text_state.update_at_line(area.height as usize, self.len());
                for ((text, color), line) in self.text.iter().skip(self.text_state.at_line).zip(area.into_iter()) {
                    let mut style = ContentStyle::new();
                    style.foreground_color = Some(*color);
                    line.render_styled(text, style, &mut gs.writer)?;
                }
            }
        }
        Ok(())
    }

    // pub fn render_at(&mut self, frame: &mut Frame, area: Rect) {
    // match self.mode {
    //     Mode::Select => {
    //         if let Some(actions) = self.actions.as_ref() {
    //             let mut list = actions.iter().map(|a| a.to_string().into()).collect::<Vec<ListItem<'static>>>();
    //             if !self.text.lines.is_empty() {
    //                 list.push("Information".into());
    //             }
    //             frame.render_stateful_widget(
    //                 List::new(list).block(BORDERED_BLOCK).highlight_style(REVERSED),
    //                 area,
    //                 &mut self.state,
    //             );
    //         }
    //     }
    //     Mode::Full => {
    //         frame.render_widget(
    //             Paragraph::new(self.text.clone())
    //                 .block(BORDERED_BLOCK)
    //                 .wrap(Wrap::default())
    //                 .scroll((self.at_line, 0)),
    //             area,
    //         );
    //     }
    // }
    // }
}

impl From<DiagnosticInfo> for Info {
    fn from(actions: DiagnosticInfo) -> Self {
        Self::from_info(actions)
    }
}

fn parse_sig_info(info: SignatureInformation, lines: &mut Vec<(String, Color)>) {
    lines.push((info.label, Color::Reset));
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
                            lines.push((String::from(line), Color::Reset));
                            // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
                        } else {
                            lines.push((String::from(line), Color::Reset));
                        }
                    }
                } else {
                    for line in c.value.lines() {
                        lines.push((String::from(line), Color::Reset));
                    }
                }
            }
            Documentation::String(s) => {
                for line in s.lines() {
                    lines.push((String::from(line), Color::Reset));
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
                    lines.push((String::from(line), Color::Reset));
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
                lines.push((line.to_owned(), Color::Reset))
                // lines.push(Line::from(generic_line(builder, usize::MAX, line, &mut ctx, Vec::new())));
            }
        }
    }
}

fn handle_markup(markup: lsp_types::MarkupContent, lines: &mut Vec<(String, Color)>) {
    if !matches!(markup.kind, lsp_types::MarkupKind::Markdown) {
        for line in markup.value.lines() {
            lines.push((line.to_owned(), Color::Reset));
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
            lines.push((line.to_owned(), Color::Reset));
        } else {
            lines.push((line.to_owned(), Color::Reset))
        }
    }
}

fn parse_markedstr(value: MarkedString) -> String {
    match value {
        MarkedString::LanguageString(data) => data.value,
        MarkedString::String(value) => value,
    }
}
