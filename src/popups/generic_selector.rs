use super::{Components, InplacePopup, PopupInterface, Status};
use crate::{
    global_state::{Clipboard, GlobalState, PopupMessage},
    render::{backend::Backend, layout::Rect, state::State},
    workspace::CursorPosition,
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

pub struct PopupSelectorX<T, R> {
    pub options: Vec<T>,
    pub state: State,
    display: fn(&T) -> &str,
    command: fn(&mut PopupSelectorX<T, R>, &mut Components) -> R,
    size: (u16, usize),
    rect: Option<Rect>,
}

impl<T, R> InplacePopup for PopupSelectorX<T, R> {
    type R = R;

    fn force_render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut rect = gs.screen_rect.center(height, width);
        let backend = gs.backend();
        rect.bordered();
        self.rect.replace(rect);
        rect.draw_borders(None, None, backend);
        match self.options.is_empty() {
            true => self.state.render_list(["No results found!"].into_iter(), rect, backend),
            false => self.state.render_list(self.options.iter().map(|opt| (self.display)(opt)), rect, backend),
        };
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        if self.options.is_empty() {
            return Status::Dropped;
        }
        match key.code {
            KeyCode::Enter => return Status::Result((self.command)(self, components)),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.state.prev(self.options.len());
                self.force_render(components.gs);
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.state.next(self.options.len());
                self.force_render(components.gs);
            }
            _ => (),
        }
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), row, column, .. } => {
                if let Some(pos) = self.rect.and_then(|rect| rect.relative_position(row, column)) {
                    let option_idx = pos.line + self.state.at_line;
                    if option_idx >= self.options.len() {
                        return Status::Pending;
                    }
                    self.state.select(option_idx, self.options.len());
                    return Status::Result((self.command)(self, components));
                }
            }
            MouseEvent { kind: MouseEventKind::Moved, row, column, .. } => {
                let (height, width) = self.size;
                let rect = components.gs.screen_rect.center(height, width);
                if let Some(pos) = rect.relative_position(row, column) {
                    let option_idx = pos.line + self.state.at_line;
                    if option_idx >= self.options.len() {
                        return Status::Pending;
                    }
                    self.state.select(option_idx, self.options.len());
                    self.force_render(components.gs);
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.state.prev(self.options.len());
                self.force_render(components.gs);
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.state.next(self.options.len());
                self.force_render(components.gs);
            }
            _ => (),
        }
        Status::Pending
    }

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        self.force_render(gs);
        true
    }
    fn render(&mut self, _: &mut GlobalState) {}
}

impl<T, R> PopupSelectorX<T, R> {
    pub fn new(
        options: Vec<T>,
        display: fn(&T) -> &str,
        command: fn(&mut PopupSelectorX<T, R>, &mut Components) -> R,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((20, 120));
        Self { options, display, command, state: State::new(), size, rect: None }
    }
}

pub fn selector_ranges(
    options: Vec<((CursorPosition, CursorPosition), String)>,
) -> PopupSelectorX<((CursorPosition, CursorPosition), String), ()> {
    PopupSelectorX::new(
        options,
        // display: |((from, _), line)| format!("({}) {line}", from.line + 1),
        |((..), line)| line,
        |popup, components| {
            let (from, to) = popup.options[popup.state.selected].0;
            if let Some(editor) = components.ws.get_active() {
                editor.go_to_select(from, to);
            }
        },
        None,
    )
}

pub fn selector_editors(options: Vec<String>) -> PopupSelectorX<String, ()> {
    PopupSelectorX::new(
        options,
        |editor| editor,
        |popup, components| {
            let Components { gs, ws, .. } = components;
            ws.activate_editor(popup.state.selected, gs);
            gs.insert_mode();
        },
        None,
    )
}
