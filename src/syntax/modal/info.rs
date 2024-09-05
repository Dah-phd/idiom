use super::ModalMessage;
use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    render::{
        backend::Style,
        layout::Rect,
        state::State,
        widgets::{StyledLine, Writable},
    },
    syntax::{theme::Theme, Action, DiagnosticInfo, Lang},
};
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
    text: Vec<StyledLine>,
    state: State,
    text_state: usize,
    mode: Mode,
}

impl Info {
    pub fn from_info(info: DiagnosticInfo) -> Self {
        let mode = if info.actions.is_some() { Mode::Select } else { Mode::Text };
        let mut text = Vec::new();
        for (msg, color) in info.messages.into_iter() {
            let style = Style::fg(color);
            for line in msg.split("\n") {
                text.push((String::from(line), style).into());
            }
        }
        Self { actions: info.actions, text, mode, ..Default::default() }
    }

    pub fn from_hover(hover: Hover, lang: &Lang, theme: &Theme) -> Self {
        let mut lines = Vec::new();
        parse_hover(hover, lang, theme, &mut lines);
        Self { text: lines, ..Default::default() }
    }

    pub fn from_signature(signature: SignatureHelp, lang: &Lang, theme: &Theme) -> Self {
        let mut lines = Vec::new();
        for info in signature.signatures {
            parse_sig_info(info, lang, theme, &mut lines);
        }
        Self { text: lines, ..Default::default() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self.mode {
            Mode::Text => self.text.len(),
            Mode::Select => self.actions.as_ref().map(|i| i.len()).unwrap_or_default() + self.text.len(),
        }
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        if self.text.is_empty() && self.actions.is_none() {
            return ModalMessage::Done;
        }
        match action {
            EditorAction::NewLine | EditorAction::Right => {
                if !matches!(self.mode, Mode::Select) {
                    return ModalMessage::Done;
                }
                if let Some(mut i) = self.actions.take() {
                    return match i.len().cmp(&self.state.selected) {
                        Ordering::Greater => {
                            gs.event.push(i.remove(self.state.selected).into());
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
            EditorAction::Up => self.prev(),
            EditorAction::Down => self.next(),
            EditorAction::Left if !matches!(self.mode, Mode::Select) && self.actions.is_some() => {
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

    pub fn push_hover(&mut self, hover: Hover, lang: &Lang, theme: &Theme) {
        parse_hover(hover, lang, theme, &mut self.text);
        self.state.selected = 0;
    }

    pub fn push_signature(&mut self, signature: SignatureHelp, lang: &Lang, theme: &Theme) {
        for info in signature.signatures {
            parse_sig_info(info, lang, theme, &mut self.text);
        }
        self.state.selected = 0;
    }

    #[inline]
    pub fn render(&mut self, area: Rect, gs: &mut GlobalState) {
        match self.mode {
            Mode::Select => {
                if let Some(actions) = self.actions.as_ref() {
                    let actions = actions.iter().map(|a| a.to_string()).collect::<Vec<_>>();
                    let options = actions.iter().map(|s| s.as_str());
                    if !self.text.is_empty() {
                        self.state.render_list(options.chain(["Information"]), &area, &mut gs.writer);
                    } else {
                        self.state.render_list(options, &area, &mut gs.writer);
                    };
                }
            }
            Mode::Text => {
                let mut lines = area.into_iter();
                let mut text = self.text.iter().skip(self.text_state);
                while lines.len() > 0 {
                    match text.next() {
                        Some(text) => text.wrap(&mut lines, &mut gs.writer),
                        None => break,
                    }
                }
                for line in lines {
                    line.render_empty(&mut gs.writer);
                }
            }
        }
    }
}

impl From<DiagnosticInfo> for Info {
    fn from(actions: DiagnosticInfo) -> Self {
        Self::from_info(actions)
    }
}

fn parse_sig_info(info: SignatureInformation, lang: &Lang, theme: &Theme, lines: &mut Vec<StyledLine>) {
    lines.push(lang.stylize(&info.label, theme));
    if let Some(text) = info.documentation {
        match text {
            Documentation::MarkupContent(c) => {
                if matches!(c.kind, lsp_types::MarkupKind::Markdown) {
                    let mut is_code = false;
                    for line in c.value.split("\n") {
                        if line.starts_with("```") {
                            is_code = !is_code;
                            continue;
                        }
                        if is_code {
                            lines.push(lang.stylize(line, theme));
                        } else {
                            lines.push(line.to_owned().into());
                        }
                    }
                } else {
                    for line in c.value.split("\n") {
                        lines.push(lang.stylize(line, theme));
                    }
                }
            }
            Documentation::String(s) => {
                for line in s.split("\n") {
                    lines.push(lang.stylize(line, theme));
                }
            }
        }
    }
}

fn parse_hover(hover: Hover, lang: &Lang, theme: &Theme, lines: &mut Vec<StyledLine>) {
    match hover.contents {
        HoverContents::Array(arr) => {
            // let mut ctx = LineBuilderContext::default();
            for value in arr {
                parse_markedstr(value, lang, theme, lines);
            }
        }
        HoverContents::Markup(markup) => {
            handle_markup(markup, lang, theme, lines);
        }
        HoverContents::Scalar(value) => {
            parse_markedstr(value, lang, theme, lines);
        }
    }
}

fn handle_markup(markup: lsp_types::MarkupContent, lang: &Lang, theme: &Theme, lines: &mut Vec<StyledLine>) {
    if !matches!(markup.kind, lsp_types::MarkupKind::Markdown) {
        for line in markup.value.split("\n") {
            lines.push(lang.stylize(line, theme));
        }
        return;
    }
    let mut is_code = false;
    for line in markup.value.split("\n") {
        if line.trim().starts_with("```") {
            is_code = !is_code;
            continue;
        }
        if is_code {
            lines.push(lang.stylize(line, theme));
        } else if line.trim().starts_with('#') {
            lines.push(line.to_owned().into());
        } else {
            lines.push(line.to_owned().into())
        }
    }
}

fn parse_markedstr(value: MarkedString, lang: &Lang, theme: &Theme, lines: &mut Vec<StyledLine>) {
    match value {
        MarkedString::LanguageString(data) => {
            for text_line in data.value.split("\n") {
                lines.push(lang.stylize(text_line, theme))
            }
        }
        MarkedString::String(value) => {
            for text_line in value.split("\n") {
                lines.push(StyledLine::from(text_line.to_owned()))
            }
        }
    }
}
