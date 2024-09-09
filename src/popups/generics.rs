use super::PopupInterface;
use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage},
    render::{
        backend::{Backend, Style},
        layout::Line,
        state::State,
        Button,
    },
};
use crossterm::event::{KeyCode, KeyEvent};

pub struct Popup {
    pub message: String,
    title: String,
    message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    buttons: Vec<Button>,
    size: (u16, usize),
    state: usize,
    updated: bool,
}

impl PopupInterface for Popup {
    fn render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut area = gs.screen_rect.center(height, width);
        area.bordered();
        area.draw_borders(None, None, &mut gs.writer);
        area.border_title(&self.title, &mut gs.writer);
        let mut lines = area.into_iter();
        if let Some(first_line) = lines.next() {
            self.p_from_message(first_line, &mut gs.writer);
        }
        if let Some(second_line) = lines.next() {
            self.spans_from_buttons(second_line, &mut gs.writer);
        }
    }

    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard) -> PopupMessage {
        if let Some(button) =
            self.buttons.iter().find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            return (button.command)(self);
        }
        match key.code {
            KeyCode::Char(ch) if self.message_as_buffer_builder.is_some() => {
                if let Some(buffer_builder) = self.message_as_buffer_builder {
                    if let Some(ch) = buffer_builder(ch) {
                        self.message.push(ch);
                    }
                }
                PopupMessage::None
            }
            KeyCode::Backspace if self.message_as_buffer_builder.is_some() => {
                self.message.pop();
                PopupMessage::None
            }
            KeyCode::Enter => (self.buttons[self.state].command)(self),
            KeyCode::Left => {
                if self.state > 0 {
                    self.state -= 1;
                } else {
                    self.state = self.buttons.len() - 1;
                }
                PopupMessage::None
            }
            KeyCode::Right => {
                if self.state < self.buttons.len() - 1 {
                    self.state += 1;
                } else {
                    self.state = 0;
                }
                PopupMessage::None
            }
            KeyCode::Esc => PopupMessage::Clear,
            _ => PopupMessage::None,
        }
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

impl Popup {
    pub fn new(
        message: String,
        title: Option<String>,
        message_as_buffer_builder: Option<fn(char) -> Option<char>>,
        buttons: Vec<Button>,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((6, 40));
        let title = title.unwrap_or("Prompt".to_owned());
        Self { message, title, message_as_buffer_builder, buttons, size, state: 0, updated: true }
    }

    pub fn with_state(mut self, idx: usize) -> Self {
        if self.buttons.len() > idx {
            self.state = idx;
        }
        self
    }

    fn p_from_message(&self, line: Line, backend: &mut Backend) {
        if self.message_as_buffer_builder.is_none() {
            return line.render_centered(&self.message, backend);
        }
        let mut builder = line.unsafe_builder(backend);
        builder.push(" >> ");
        builder.push(&self.message);
        builder.push_styled("|", Style::slowblink());
    }

    fn spans_from_buttons(&self, line: Line, backend: &mut Backend) {
        let btn_count = self.buttons.len();
        let sum_btn_names_len: usize = self.buttons.iter().map(|b| b.name.len()).sum();
        let padding = line.width.saturating_sub(sum_btn_names_len) / btn_count;
        let mut builder = line.unsafe_builder(backend);
        for (idx, btn) in self.buttons.iter().enumerate() {
            let text = format!("{name:^width$}", name = btn.name, width = padding + btn.name.len());
            if idx == self.state {
                if !builder.push_styled(text.as_str(), Style::reversed()) {
                    break;
                }
            } else if !builder.push(text.as_str()) {
                break;
            };
        }
    }
}

pub struct PopupSelector<T> {
    pub options: Vec<T>,
    pub state: State,
    display: fn(&T) -> &str,
    command: fn(&mut PopupSelector<T>) -> PopupMessage,
    size: (u16, usize),
    updated: bool,
}

impl<T> PopupInterface for PopupSelector<T> {
    fn render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut rect = gs.screen_rect.center(height, width);
        rect.bordered();
        rect.draw_borders(None, None, &mut gs.writer);
        if self.options.is_empty() {
            self.state.render_list(["No results found!"].into_iter(), &rect, &mut gs.writer);
        } else {
            self.state.render_list(self.options.iter().map(|opt| (self.display)(opt)), &rect, &mut gs.writer);
        };
    }

    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard) -> PopupMessage {
        if self.options.is_empty() {
            return PopupMessage::Clear;
        }
        match key.code {
            KeyCode::Enter => (self.command)(self),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.options.len());
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.options.len());
                PopupMessage::None
            }
            _ => PopupMessage::None,
        }
    }

    fn mark_as_updated(&mut self) {
        self.updated = true;
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

impl<T> PopupSelector<T> {
    pub fn new(
        options: Vec<T>,
        display: fn(&T) -> &str,
        command: fn(&mut PopupSelector<T>) -> PopupMessage,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((20, 120));
        Self { options, display, command, state: State::new(), size, updated: true }
    }

    pub fn with_state(mut self, idx: usize) -> Self {
        self.state.select(idx, 1);
        self
    }
}
