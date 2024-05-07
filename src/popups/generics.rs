use std::io::Write;

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
    pub title: Option<String>,
    pub message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    pub buttons: Vec<Button>,
    pub size: Option<(u16, usize)>,
    pub state: usize,
}

impl PopupInterface for Popup {
    fn render(&mut self, gs: &mut GlobalState) -> std::io::Result<()> {
        let (height, width) = self.size.unwrap_or((6, 40));
        let mut area = gs.screen_rect.center(height, width);
        area.bordered();
        area.draw_borders(None, None, &mut gs.writer)?;
        area.border_title(self.title(), &mut gs.writer)?;
        let mut lines = area.into_iter();
        if let Some(first_line) = lines.next() {
            self.p_from_message(first_line, &mut gs.writer)?;
        }
        if let Some(second_line) = lines.next() {
            self.spans_from_buttons(second_line, &mut gs.writer)?;
        }
        gs.writer.flush()
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
}

impl Popup {
    fn p_from_message(&self, line: Line, backend: &mut Backend) -> std::io::Result<()> {
        if self.message_as_buffer_builder.is_none() {
            return line.render_centered(&self.message, backend);
        }
        let mut builder = line.unsafe_builder(backend)?;
        builder.push(" >> ")?;
        builder.push(&self.message)?;
        builder.push_styled("|", Style::slowblink())?;
        Ok(())
    }

    fn title(&self) -> String {
        if let Some(title) = &self.title {
            return format!("{title} ");
        }
        "Prompt".to_owned()
    }

    fn spans_from_buttons(&self, line: Line, backend: &mut Backend) -> std::io::Result<()> {
        let btn_count = self.buttons.len();
        let sum_btn_names_len: usize = self.buttons.iter().map(|b| b.name.len()).sum();
        let padding = line.width.saturating_sub(sum_btn_names_len) / btn_count;
        let mut builder = line.unsafe_builder(backend)?;
        for btn in self.buttons.iter() {
            if !builder.push(format!("{name:^width$}", name = btn.name, width = padding + btn.name.len()).as_str())? {
                break;
            }
        }
        Ok(())
    }
}

pub struct PopupSelector<T> {
    pub options: Vec<T>,
    pub display: fn(&T) -> &str,
    pub command: fn(&mut PopupSelector<T>) -> PopupMessage,
    pub state: State,
    pub size: Option<(u16, usize)>,
}

impl<T> PopupInterface for PopupSelector<T> {
    fn render(&mut self, gs: &mut GlobalState) -> std::io::Result<()> {
        let (height, width) = self.size.unwrap_or((20, 120));
        let rect = gs.screen_rect.center(height, width);

        if self.options.is_empty() {
            self.state.render_list(["No results found!"].into_iter(), &rect, &mut gs.writer)?;
        } else {
            self.state.render_list(self.options.iter().map(|opt| (self.display)(opt)), &rect, &mut gs.writer)?;
        };
        gs.writer.flush()
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
            KeyCode::Esc => PopupMessage::Clear,
            _ => PopupMessage::None,
        }
    }
}
