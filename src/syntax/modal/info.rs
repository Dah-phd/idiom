use super::ModalMessage;
use crate::{
    configs::{EditorAction, Theme},
    ext_tui::{State, StyleExt, StyledLine},
    global_state::GlobalState,
    lsp::Highlighter,
    syntax::{DiagnosticInfo, Fix},
};
use crossterm::style::ContentStyle;
use idiom_tui::{
    layout::{IterLines, Rect},
    widgets::Writable,
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
    style_builder: Option<Highlighter>,
    actions: Option<Vec<Fix>>,
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
            let style = ContentStyle::fg(color);
            for line in msg.split("\n") {
                text.push((String::from(line), style).into());
            }
        }
        Self { actions: info.actions, text, mode, ..Default::default() }
    }

    pub fn from_hover(hover: Hover, theme: &Theme) -> Self {
        let mut lines = Vec::new();
        let mut sty = Highlighter::new(theme);
        parse_hover(hover, &mut sty, &mut lines);
        Self { text: lines, style_builder: Some(sty), ..Default::default() }
    }

    pub fn from_signature(signature: SignatureHelp, theme: &Theme) -> Self {
        let mut lines = Vec::new();
        let mut sty = Highlighter::new(theme);
        for info in signature.signatures {
            parse_sig_info(info, &mut sty, &mut lines);
        }
        Self { text: lines, style_builder: Some(sty), ..Default::default() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self.mode {
            Mode::Text => self.text.len(),
            Mode::Select if self.text.is_empty() => self.actions.as_ref().map(Vec::len).unwrap_or_default(),
            Mode::Select => self.actions.as_ref().map(Vec::len).unwrap_or_default() + 1,
        }
    }

    pub fn map(&mut self, action: EditorAction, gs: &mut GlobalState) -> ModalMessage {
        if self.text.is_empty() && self.actions.is_none() {
            return ModalMessage::Done;
        }
        match action {
            EditorAction::NewLine | EditorAction::Right => self.finish(gs),
            EditorAction::Up | EditorAction::ScrollUp => self.prev(),
            EditorAction::Down | EditorAction::ScrollDown => self.next(),
            EditorAction::Left if matches!(self.mode, Mode::Text) && self.actions.is_some() => {
                self.mode = Mode::Select;
                self.state.select(0, self.len());
                ModalMessage::Taken
            }
            _ => ModalMessage::Done,
        }
    }

    pub fn mouse_moved(&mut self, rel_index: usize) -> bool {
        match self.mode {
            Mode::Text => false,
            Mode::Select => {
                let expected_select = self.state.at_line + rel_index;
                if expected_select == self.state.selected {
                    return false;
                }
                self.state.select(expected_select, self.len());
                true
            }
        }
    }

    pub fn mouse_click_and_finish(&mut self, rel_index: usize, gs: &mut GlobalState) -> bool {
        match self.mode {
            Mode::Text => true,
            Mode::Select => {
                let selected = self.state.at_line + rel_index;
                self.state.select(selected, self.len());
                !matches!(self.finish(gs), ModalMessage::Taken | ModalMessage::None)
            }
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

    pub fn push_hover(&mut self, hover: Hover, theme: &Theme) {
        self.text.push(StyledLine::default());
        match self.style_builder.as_mut() {
            Some(sty) => parse_hover(hover, sty, &mut self.text),
            None => {
                let mut sty = Highlighter::new(theme);
                parse_hover(hover, &mut sty, &mut self.text);
                self.style_builder.replace(sty);
            }
        }
        self.state.selected = 0;
    }

    pub fn push_signature(&mut self, signature: SignatureHelp, theme: &Theme) {
        self.text.push(StyledLine::default());
        match self.style_builder.as_mut() {
            Some(sty) => {
                for info in signature.signatures {
                    parse_sig_info(info, sty, &mut self.text);
                }
            }
            None => {
                let mut sty = Highlighter::new(theme);
                for info in signature.signatures {
                    parse_sig_info(info, &mut sty, &mut self.text);
                }
                self.style_builder.replace(sty);
            }
        }
        self.state.selected = 0;
    }

    #[inline]
    pub fn render(&mut self, area: Rect, gs: &mut GlobalState) {
        match self.mode {
            Mode::Select => self.render_select(area, gs),
            Mode::Text => self.render_text(area, gs),
        }
    }

    fn render_select(&mut self, area: Rect, gs: &mut GlobalState) {
        let actions = match self.actions.as_ref() {
            None => return,
            Some(actions) => actions.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        };

        let options = actions.iter().map(String::as_str);
        let lines = area.iter_padded(1);

        match self.text.is_empty() {
            true => self.state.render_list_padded(options, lines, &mut gs.backend),
            false => self.state.render_list_padded(options.chain(["Information"]), lines, &mut gs.backend),
        };
    }

    fn render_text(&mut self, area: Rect, gs: &mut GlobalState) {
        let mut lines = area.iter_padded(1);
        let mut text = self.text.iter().skip(self.text_state);
        while lines.len() > 0 {
            match text.next() {
                Some(text) => text.wrap(&mut lines, &mut gs.backend),
                None => break,
            }
        }
        lines.clear_to_end(&mut gs.backend);
    }

    fn finish(&mut self, gs: &mut GlobalState) -> ModalMessage {
        if !matches!(self.mode, Mode::Select) {
            return ModalMessage::Done;
        }
        if let Some(actions) = self.actions.as_mut() {
            return match actions.len().cmp(&self.state.selected) {
                Ordering::Greater => {
                    gs.event.push(actions.remove(self.state.selected).into());
                    ModalMessage::TakenDone
                }
                _ => {
                    self.mode = Mode::Text;
                    self.state.select(0, self.len());
                    ModalMessage::Taken
                }
            };
        }
        ModalMessage::Done
    }
}

impl From<DiagnosticInfo> for Info {
    fn from(actions: DiagnosticInfo) -> Self {
        Self::from_info(actions)
    }
}

fn parse_sig_info(info: SignatureInformation, sty: &mut Highlighter, lines: &mut Vec<StyledLine>) {
    lines.push(sty.parse_line(&info.label));
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
                            lines.push(sty.parse_line(line));
                        } else {
                            lines.push(line.to_owned().into());
                        }
                    }
                } else {
                    for line in c.value.split("\n") {
                        lines.push(sty.parse_line(line));
                    }
                }
            }
            Documentation::String(s) => {
                for line in s.split("\n") {
                    lines.push(sty.parse_line(line));
                }
            }
        }
    }
}

fn parse_hover(hover: Hover, sty: &mut Highlighter, lines: &mut Vec<StyledLine>) {
    match hover.contents {
        HoverContents::Array(arr) => {
            // let mut ctx = LineBuilderContext::default();
            for value in arr {
                parse_markedstr(value, sty, lines);
            }
        }
        HoverContents::Markup(markup) => {
            handle_markup(markup, sty, lines);
        }
        HoverContents::Scalar(value) => {
            parse_markedstr(value, sty, lines);
        }
    }
}

fn handle_markup(markup: lsp_types::MarkupContent, sty: &mut Highlighter, lines: &mut Vec<StyledLine>) {
    if !matches!(markup.kind, lsp_types::MarkupKind::Markdown) {
        for line in markup.value.split("\n") {
            lines.push(sty.parse_line(line));
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
            lines.push(sty.parse_line(line));
        } else if line.trim().starts_with('#') {
            lines.push(line.to_owned().into());
        } else {
            lines.push(line.to_owned().into())
        }
    }
}

fn parse_markedstr(value: MarkedString, sty: &mut Highlighter, lines: &mut Vec<StyledLine>) {
    match value {
        MarkedString::LanguageString(data) => {
            for text_line in data.value.split("\n") {
                lines.push(sty.parse_line(text_line));
            }
        }
        MarkedString::String(value) => {
            for text_line in value.split("\n") {
                lines.push(StyledLine::from(text_line.to_owned()))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Info, ModalMessage, Mode};
    use crate::configs::{EditorAction, Theme};
    use crate::ext_tui::CrossTerm;
    use crate::global_state::GlobalState;
    use crate::syntax::{DiagnosticInfo, Fix};
    use idiom_tui::{layout::Rect, Backend};
    use lsp_types::Hover;

    #[test]
    fn mouse() {
        let second = Fix::Import("second".to_owned());
        let di =
            DiagnosticInfo { actions: Some(vec![Fix::Import("first".to_owned()), second.clone()]), messages: vec![] };
        let theme = Theme::default();
        let mut gs = GlobalState::new(Rect::default(), CrossTerm::init());

        let mut modal = Info::from(di);
        modal.push_hover(
            Hover {
                contents: lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String("test text".to_owned())),
                range: None,
            },
            &theme,
        );

        assert_eq!(0, modal.state.selected);
        assert!(modal.mouse_moved(1));
        assert_eq!(1, modal.state.selected);
        assert!(modal.actions.is_some());
        assert!(!modal.mouse_click_and_finish(2, &mut gs));
        assert!(modal.actions.is_some());
        assert!(matches!(modal.mode, Mode::Text));
        assert_eq!(modal.state.selected, 0);
        assert!(matches!(modal.map(EditorAction::Left, &mut gs), ModalMessage::Taken));
        assert!(matches!(modal.mode, Mode::Select));
        assert!(modal.mouse_moved(2));
        assert_eq!(modal.state.selected, 2);
        assert!(modal.mouse_click_and_finish(1, &mut gs));
        assert_eq!(gs.event, vec![second.into()]);
    }
}
