use crate::render::{
    backend::{Backend, StyleExt},
    layout::Line,
};
use crate::{
    embeded_term::EditorTerminal,
    global_state::GlobalState,
    popups::{Components, Popup, Status},
    tree::Tree,
    workspace::Workspace,
};
use crossterm::{
    event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    style::ContentStyle,
};
use std::ops::Range;

#[derive(Clone, PartialEq)]
pub struct CommandButton {
    pub command: fn(&mut PopupChoice, &mut Components),
    pub name: &'static str,
    pub key: Option<Vec<KeyCode>>,
}

impl std::fmt::Debug for CommandButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("").field(&self.name).finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopupChoice {
    pub message: String,
    title_prefix: Option<&'static str>,
    title: String,
    message_as_buffer_builder: Option<fn(char) -> Option<char>>,
    buttons: Vec<CommandButton>,
    button_line: u16,
    button_ranges: Vec<Range<u16>>,
    size: (u16, usize),
    state: usize,
    updated: bool,
}

impl PopupChoice {
    fn mark_as_updated(&mut self) {
        self.updated = true;
    }

    fn collect_update_status(&mut self) -> bool {
        std::mem::take(&mut self.updated)
    }
}

impl Popup for PopupChoice {
    type R = ();

    fn render(&mut self, gs: &mut GlobalState) {
        if self.collect_update_status() {
            self.force_render(gs);
        }
    }

    fn force_render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut area = gs.screen_rect.center(height, width);
        let backend = gs.backend();
        area.bordered();
        area.draw_borders(None, None, backend);
        match self.title_prefix {
            Some(prefix) => area.border_title_prefixed(prefix, &self.title, backend),
            None => area.border_title(&self.title, backend),
        };
        let mut lines = area.into_iter();
        if let Some(first_line) = lines.next() {
            self.p_from_message(first_line, backend);
        }
        if let Some(second_line) = lines.next() {
            self.spans_from_buttons(second_line, backend);
        }
    }

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        self.mark_as_updated();
        if let Some(button) =
            self.buttons.iter().find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            (button.command)(self, components);
            return Status::Result(());
        }
        match key.code {
            KeyCode::Char(ch) if self.message_as_buffer_builder.is_some() => {
                if let Some(buffer_builder) = self.message_as_buffer_builder {
                    if let Some(ch) = buffer_builder(ch) {
                        self.message.push(ch);
                    }
                }
            }
            KeyCode::Backspace if self.message_as_buffer_builder.is_some() => {
                self.message.pop();
            }
            KeyCode::Enter => {
                (self.buttons[self.state].command)(self, components);
                return Status::Result(());
            }
            KeyCode::Left => {
                self.prev();
            }
            KeyCode::Right => {
                self.next();
            }
            _ => (),
        }
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status<Self::R> {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } if row == self.button_line => {
                if let Some(position) = self.button_ranges.iter().position(|btn_range| btn_range.contains(&column)) {
                    (self.buttons[position].command)(self, components);
                    return Status::Result(());
                }
            }
            MouseEvent { kind: MouseEventKind::Moved, column, row, .. } if row == self.button_line => {
                if let Some(position) = self.button_ranges.iter().position(|btn_range| btn_range.contains(&column)) {
                    self.state = position;
                    self.mark_as_updated();
                }
            }
            MouseEvent { kind: MouseEventKind::ScrollUp, .. } => {
                self.prev();
                self.mark_as_updated();
            }
            MouseEvent { kind: MouseEventKind::ScrollDown, .. } => {
                self.next();
                self.mark_as_updated();
            }
            _ => (),
        }
        Status::Pending
    }

    fn resize_success(&mut self, _gs: &mut GlobalState) -> bool {
        true
    }
}

impl PopupChoice {
    pub fn new(
        message: String,
        title_prefix: Option<&'static str>,
        title: Option<String>,
        message_as_buffer_builder: Option<fn(char) -> Option<char>>,
        buttons: Vec<CommandButton>,
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
        builder.push_styled("|", ContentStyle::slowblink());
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
                if !builder.push_styled(text.as_str(), ContentStyle::reversed()) {
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

pub fn save_and_exit_popup(
    gs: &mut GlobalState,
    ws: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> bool {
    PopupChoice::new(
        "Not all opened editors are saved!".into(),
        None,
        None,
        None,
        vec![
            CommandButton {
                command: |_, c| c.ws.save_all(c.gs),
                name: "Save All (Y)",
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            CommandButton {
                command: |_, _| (),
                name: "Don't save (N)",
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        Some((4, 40)),
    )
    .run(gs, ws, tree, term)
    .is_some()
}
