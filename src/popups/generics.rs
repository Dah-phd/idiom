use std::ops::Range;

use super::PopupInterface;
use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage},
    render::{
        backend::{Backend, Style},
        layout::{Line, Rect},
        state::State,
        Button,
    },
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct Popup {
    pub message: String,
    title_prefix: Option<&'static str>,
    title: String,
    message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    buttons: Vec<Button>,
    button_line: u16,
    button_ranges: Vec<Range<u16>>,
    size: (u16, usize),
    state: usize,
    updated: bool,
}

impl PopupInterface for Popup {
    fn render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut area = gs.screen_rect.center(height, width);
        area.bordered();
        area.draw_borders(None, None, gs.backend());
        match self.title_prefix {
            Some(prefix) => area.border_title_prefixed(prefix, &self.title, gs.backend()),
            None => area.border_title(&self.title, gs.backend()),
        };
        let mut lines = area.into_iter();
        if let Some(first_line) = lines.next() {
            self.p_from_message(first_line, gs.backend());
        }
        if let Some(second_line) = lines.next() {
            self.spans_from_buttons(second_line, &mut gs.writer);
        }
    }

    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
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
                self.prev();
                PopupMessage::None
            }
            KeyCode::Right => {
                self.next();
                PopupMessage::None
            }
            KeyCode::Esc => PopupMessage::Clear,
            _ => PopupMessage::None,
        }
    }

    fn mouse_map(&mut self, event: MouseEvent) -> PopupMessage {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } if row == self.button_line => {
                if let Some(position) = self.button_ranges.iter().position(|btn_range| btn_range.contains(&column)) {
                    return (self.buttons[position].command)(self);
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.mark_as_updated();
                self.prev();
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.mark_as_updated();
                self.next();
            }
            _ => (),
        }
        PopupMessage::None
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
        title_prefix: Option<&'static str>,
        title: Option<String>,
        message_as_buffer_builder: Option<fn(char) -> Option<char>>,
        buttons: Vec<Button>,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((6, 40));
        let title = title.unwrap_or("Prompt".to_owned());
        Self {
            message,
            title_prefix,
            title,
            message_as_buffer_builder,
            buttons,
            button_line: 0,
            button_ranges: vec![],
            size,
            state: 0,
            updated: true,
        }
    }

    #[allow(dead_code)]
    pub fn message(message: String) -> Box<Self> {
        Box::new(Self {
            message,
            title_prefix: None,
            title: "Info".to_owned(),
            message_as_buffer_builder: None,
            buttons: vec![Button { command: |_| PopupMessage::Clear, name: "Ok", key: None }],
            button_line: 0,
            button_ranges: vec![],
            size: (6, 40),
            state: 0,
            updated: true,
        })
    }

    fn next(&mut self) {
        if self.state < self.buttons.len() - 1 {
            self.state += 1;
        } else {
            self.state = 0;
        }
    }

    fn prev(&mut self) {
        if self.state > 0 {
            self.state -= 1;
        } else {
            self.state = self.buttons.len() - 1;
        }
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

    fn spans_from_buttons(&mut self, line: Line, backend: &mut Backend) {
        let mut last_btn_end = line.col;
        self.button_line = line.row;
        self.button_ranges.clear();

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
            let btn_end = last_btn_end + text.len() as u16;
            let but_range = last_btn_end..btn_end;
            last_btn_end = btn_end;
            self.button_ranges.push(but_range)
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
    rect: Option<Rect>,
}

impl<T> PopupInterface for PopupSelector<T> {
    fn render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut rect = gs.screen_rect.center(height, width);
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, &mut gs.writer);
        match self.options.is_empty() {
            true => self.state.render_list(["No results found!"].into_iter(), rect, &mut gs.writer),
            false => self.state.render_list(self.options.iter().map(|opt| (self.display)(opt)), rect, &mut gs.writer),
        };
    }

    fn key_map(&mut self, key: &KeyEvent, _: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
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

    fn mouse_map(&mut self, event: MouseEvent) -> PopupMessage {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), row, column, .. } => {
                if let Some(pos) = self.rect.and_then(|rect| rect.relative_position(row, column)) {
                    let option_idx = pos.line + self.state.at_line;
                    if option_idx >= self.options.len() {
                        return PopupMessage::None;
                    }
                    self.state.select(option_idx, self.options.len());
                    self.mark_as_updated();
                    return (self.command)(self);
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.options.len());
                self.mark_as_updated();
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.options.len());
                self.mark_as_updated();
            }
            _ => (),
        }
        PopupMessage::None
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
        Self { options, display, command, state: State::new(), size, updated: true, rect: None }
    }
}

impl PopupSelector<String> {
    #[allow(dead_code)]
    pub fn message_list<T: ToString>(list: Vec<T>) -> Box<Self> {
        let options = list.into_iter().map(|el| el.to_string()).collect();
        let size = (20, 120);
        Box::new(Self {
            options,
            display: |el| el.as_str(),
            command: |_| PopupMessage::Clear,
            state: State::new(),
            size,
            updated: true,
            rect: None,
        })
    }
}
