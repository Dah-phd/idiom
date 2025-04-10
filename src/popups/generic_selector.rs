use super::PopupInterface;
use crate::{
    global_state::{Clipboard, PopupMessage},
    render::{backend::Backend, layout::Rect, state::State},
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;

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
    fn render(&mut self, screen: Rect, backend: &mut Backend) {
        let (height, width) = self.size;
        let mut rect = screen.center(height, width);
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, backend);
        match self.options.is_empty() {
            true => self.state.render_list(["No results found!"].into_iter(), rect, backend),
            false => self.state.render_list(self.options.iter().map(|opt| (self.display)(opt)), rect, backend),
        };
    }

    fn resize(&mut self, _new_screen: Rect) -> PopupMessage {
        self.mark_as_updated();
        PopupMessage::None
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
