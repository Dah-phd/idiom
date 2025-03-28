use super::{
    utils::{into_message, next_option, prev_option},
    PopupInterface,
};
use crate::{
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{
        backend::{BackendProtocol, StyleExt},
        count_as_string, TextField,
    },
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::ContentStyle;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct GoToLinePopup {
    line_idx: String,
    updated: bool,
}

impl Default for GoToLinePopup {
    fn default() -> Self {
        Self { line_idx: String::default(), updated: true }
    }
}

impl GoToLinePopup {
    pub fn new() -> Box<Self> {
        Box::default()
    }

    fn parse(&mut self) -> PopupMessage {
        if self.line_idx.is_empty() {
            return PopupMessage::None;
        }
        match self.line_idx.parse::<usize>() {
            Ok(idx) => PopupMessage::Event(IdiomEvent::GoToLine { line: idx.saturating_sub(1), clear_popup: false }),
            _ => PopupMessage::None,
        }
    }
}

impl PopupInterface for GoToLinePopup {
    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        match key.code {
            KeyCode::Char(ch) if ch.is_numeric() => {
                self.line_idx.push(ch);
                self.parse()
            }
            KeyCode::Backspace if self.line_idx.pop().is_some() => self.parse(),
            KeyCode::Backspace => PopupMessage::None,
            _ => PopupMessage::Clear,
        }
    }

    fn render(&mut self, gs: &mut GlobalState) {
        if let Some(line) = gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            gs.writer.set_style(gs.theme.accent_style);
            {
                let mut builder = line.unsafe_builder(&mut gs.writer);
                builder.push(" Go to >> ");
                builder.push(&self.line_idx);
                builder.push_styled("|", ContentStyle::slowblink());
            }
            gs.writer.reset_style();
        };
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<PopupMessage>,
    pub state: usize,
}

impl FindPopup {
    pub fn new() -> Box<Self> {
        Box::new(Self { options: Vec::new(), pattern: TextField::with_editor_access(String::new()), state: 0 })
    }
}

impl PopupInterface for FindPopup {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        if matches!(key.code, KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL)) {
            return IdiomEvent::FindToReplace(self.pattern.text.to_owned(), self.options.clone()).into();
        }
        if let Some(event) = self.pattern.map(key, clipboard) {
            return event;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Down => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc | KeyCode::Left => PopupMessage::Clear,
            KeyCode::Tab => IdiomEvent::FindSelector(self.pattern.text.to_owned()).into(),
            _ => PopupMessage::None,
        }
    }

    fn render(&mut self, gs: &mut GlobalState) {
        self.fast_render(gs);
    }

    fn fast_render(&mut self, gs: &mut GlobalState) {
        if let Some(line) = gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            gs.writer.set_style(gs.theme.accent_style);
            let mut builder = line.unsafe_builder(&mut gs.writer);
            builder.push(" Found(");
            builder.push(&count_as_string(self.options.len()));
            builder.push(") >> ");
            self.pattern.insert_formatted_text(builder);
            gs.writer.reset_style();
        }
    }

    fn component_access(&mut self, ws: &mut Workspace, _tree: &mut Tree) {
        if let Some(editor) = ws.get_active() {
            self.options.clear();
            editor.find(self.pattern.text.as_str(), &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
    }

    fn mark_as_updated(&mut self) {}

    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        self.pattern.paste_passthrough(clip)
    }
}
