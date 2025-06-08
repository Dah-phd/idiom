use super::{Components, Popup, Status};
use crate::{
    ext_tui::{CrossTerm, State},
    global_state::GlobalState,
};
use crossterm::{
    event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    style::ContentStyle,
};
use idiom_tui::{
    layout::{IterLines, Line, Rect},
    Backend,
};

#[derive(PartialEq, Debug, Clone)]
pub struct PopupSelector<T> {
    pub options: Vec<T>,
    pub state: State,
    display: fn(&T, Line, &mut CrossTerm),
    command: fn(&mut PopupSelector<T>, &mut Components),
    size: (u16, usize),
}

impl<T> Popup for PopupSelector<T> {
    fn force_render(&mut self, gs: &mut GlobalState) {
        let rect = self.get_rect(gs);
        let backend = gs.backend();
        rect.draw_borders(None, None, backend);
        if self.options.is_empty() {
            self.state.render_list(["No results found!"].into_iter(), rect, backend);
            return;
        }
        self.state.update_at_line(rect.height as usize);
        let mut lines = rect.into_iter();
        for (idx, text) in self.options.iter().enumerate().skip(self.state.at_line) {
            let Some(line) = lines.next() else { break };
            match idx == self.state.selected {
                true => {
                    let reset_style = backend.get_style();
                    backend.set_style(self.state.highlight);
                    (self.display)(text, line, backend);
                    backend.set_style(reset_style);
                }
                false => {
                    (self.display)(text, line, backend);
                }
            }
        }
        lines.clear_to_end(backend);
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        if self.options.is_empty() {
            return Status::Finished;
        }
        match key.code {
            KeyCode::Enter => {
                (self.command)(self, components);
                return Status::Finished;
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::BackTab => {
                self.state.prev(self.options.len());
                self.force_render(components.gs);
            }
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Tab => {
                self.state.next(self.options.len());
                self.force_render(components.gs);
            }
            _ => (),
        }
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), row, column, .. } => {
                if let Some(pos) = self.get_rect(components.gs).relative_position(row, column) {
                    let option_idx = pos.row as usize + self.state.at_line;
                    if option_idx >= self.options.len() {
                        return Status::Pending;
                    }
                    self.state.select(option_idx, self.options.len());
                    return {
                        (self.command)(self, components);
                        Status::Finished
                    };
                }
            }
            MouseEvent { kind: MouseEventKind::Moved, row, column, .. } => {
                if let Some(pos) = self.get_rect(components.gs).relative_position(row, column) {
                    let option_idx = pos.row as usize + self.state.at_line;
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

impl<T> PopupSelector<T> {
    pub fn new(
        options: Vec<T>,
        display: fn(&T, Line, &mut CrossTerm),
        command: fn(&mut PopupSelector<T>, &mut Components),
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((20, 120));
        Self { options, display, command, state: State::new(), size }
    }

    fn get_rect(&self, gs: &GlobalState) -> Rect {
        let (height, width) = self.size;
        gs.screen_rect.center(height, width).with_borders()
    }
}

impl PopupSelector<String> {
    pub fn message_list<T: ToString>(list: Vec<T>) -> Self {
        let options = list.into_iter().map(|el| el.to_string()).collect();
        Self {
            options,
            display: |el, line, backend| line.render(el, backend),
            command: |_, _| (),
            state: State::with_highlight(ContentStyle::default()),
            size: (20, 120),
        }
    }
}
