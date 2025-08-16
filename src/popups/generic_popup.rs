use crate::{
    embeded_term::EditorTerminal,
    ext_tui::{text_field::TextField, CrossTerm, StyleExt},
    global_state::GlobalState,
    popups::{Components, Popup, Status},
    tree::Tree,
    workspace::{Workspace, FILE_STATUS_ERR},
};
use crossterm::{
    event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    style::ContentStyle,
};
use idiom_tui::layout::Line;
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
    pub message: TextField<()>,
    buffer_message: bool,
    title_prefix: Option<&'static str>,
    title: String,
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

    pub fn get_message(&self) -> &str {
        &self.message.text
    }
}

impl Popup for PopupChoice {
    fn render(&mut self, gs: &mut GlobalState) {
        if self.collect_update_status() {
            self.force_render(gs);
        }
    }

    fn force_render(&mut self, gs: &mut GlobalState) {
        let (height, width) = self.size;
        let mut area = gs.screen().center(height, width);
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

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        self.mark_as_updated();
        if let Some(button) =
            self.buttons.iter().find(|button| matches!(&button.key, Some(key_code) if key_code.contains(&key.code)))
        {
            (button.command)(self, components);
            return Status::Finished;
        }
        if self.buffer_message && self.message.map(&key, &mut components.gs.clipboard).is_some() {
            return Status::Pending;
        }
        match key.code {
            KeyCode::Enter => {
                (self.buttons[self.state].command)(self, components);
                return Status::Finished;
            }
            KeyCode::Left | KeyCode::BackTab => {
                self.prev();
            }
            KeyCode::Right | KeyCode::Tab => {
                self.next();
            }
            _ => (),
        }
        Status::Pending
    }

    fn map_mouse(&mut self, event: MouseEvent, components: &mut Components) -> Status {
        match event {
            MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column, row, .. } if row == self.button_line => {
                if let Some(position) = self.button_ranges.iter().position(|btn_range| btn_range.contains(&column)) {
                    (self.buttons[position].command)(self, components);
                    return Status::Finished;
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
    pub fn new_static(
        message: String,
        title_prefix: Option<&'static str>,
        title: Option<String>,
        buttons: Vec<CommandButton>,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((6, 40));
        let title = title.unwrap_or("Prompt".to_owned());
        let message = TextField::basic(message);
        Self {
            message,
            title_prefix,
            title,
            buffer_message: false,
            buttons,
            button_line: 0,
            button_ranges: vec![],
            size,
            state: 0,
            updated: true,
        }
    }

    pub fn new_with_text_field(
        message: String,
        title_prefix: Option<&'static str>,
        title: Option<String>,
        buttons: Vec<CommandButton>,
        size: Option<(u16, usize)>,
    ) -> Self {
        let size = size.unwrap_or((6, 40));
        let title = title.unwrap_or("Prompt".to_owned());
        let message = TextField::basic(message);
        Self {
            message,
            title_prefix,
            title,
            buffer_message: true,
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

    fn p_from_message(&self, line: Line, backend: &mut CrossTerm) {
        if self.buffer_message {
            self.message.widget(line, backend);
            return;
        }
        line.render_centered(self.get_message(), backend);
    }

    fn spans_from_buttons(&mut self, line: Line, backend: &mut CrossTerm) {
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

/// uses workaround in order to message if the popup should trigger exit
/// the solution is no ideal but it otherwise a whole messaging system will be needed
/// or different exit strategy
pub fn save_and_exit(gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) -> bool {
    if ws.iter().all(|e| gs.unwrap_or_default(e.is_saved(), FILE_STATUS_ERR)) {
        return true;
    };
    let mut popup = PopupChoice::new_static(
        "Not all opened editors are saved!".into(),
        None,
        None,
        vec![
            CommandButton {
                command: |p, c| {
                    // set prefix to true to message outside popup
                    // if button is called prefix should be some
                    p.title_prefix = Some("");
                    c.ws.save_all(c.gs);
                },
                name: "Save All (Y)",
                key: Some(vec![KeyCode::Char('y'), KeyCode::Char('Y')]),
            },
            CommandButton {
                // set prefix to true to message outside
                // if button is called prefix should be some
                command: |p, _| p.title_prefix = Some(""),
                name: "Don't save (N)",
                key: Some(vec![KeyCode::Char('n'), KeyCode::Char('N')]),
            },
        ],
        Some((4, 40)),
    );

    if let Err(error) = popup.run(gs, ws, tree, term) {
        gs.error(error);
    };

    // ! check if prefix was set during popup execution
    popup.title_prefix.is_some()
}
