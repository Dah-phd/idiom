use super::{
    generic_selector::PopupSelector,
    popup_replace::ReplacePopup,
    utils::{next_option, prev_option},
    Components, Popup, Status,
};
use crate::{
    embeded_term::EditorTerminal,
    ext_tui::{text_field::TextField, StyleExt},
    global_state::GlobalState,
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::style::ContentStyle;
use idiom_tui::{
    count_as_string,
    layout::{Line, Rect},
    Backend,
};

pub struct GoToLinePopup {
    current_line: usize,
    line_idx: String,
    render_line: Line,
    accent: ContentStyle,
}

impl GoToLinePopup {
    pub fn run_inplace(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let Some(editor) = workspace.get_active() else { return };
        let current_line = editor.cursor.line;
        let Some(mut popup) = GoToLinePopup::new(current_line, *gs.editor_area(), gs.ui_theme.accent_style()) else {
            return;
        };
        if let Err(error) = popup.run(gs, workspace, tree, term) {
            gs.error(error);
        }
    }

    pub fn new(current_line: usize, editor_area: Rect, accent: ContentStyle) -> Option<Self> {
        let render_line = editor_area.right_top_corner(1, 50).into_iter().next()?;
        Some(Self { current_line, line_idx: String::default(), render_line, accent })
    }

    fn parse(&mut self) -> Option<usize> {
        if self.line_idx.is_empty() {
            return Some(self.current_line);
        };
        self.line_idx.parse().ok()
    }
}

impl Popup for GoToLinePopup {
    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        let Components { gs, ws, .. } = components;

        let result_idx = match key.code {
            KeyCode::Enter => return Status::Finished,
            KeyCode::Char(ch) if ch.is_numeric() => {
                self.line_idx.push(ch);
                self.parse()
            }
            KeyCode::Backspace if self.line_idx.pop().is_some() => self.parse(),
            _ => return Status::Pending,
        };
        if let Some(line) = result_idx {
            let Some(editor) = ws.get_active() else {
                return Status::Finished;
            };
            editor.go_to(line);
            gs.backend.freeze();
            editor.render(gs);
            self.force_render(gs);
            gs.backend.unfreeze();
        }
        Status::Pending
    }

    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset_style = backend.get_style();
        backend.set_style(self.accent);
        {
            let mut builder = self.render_line.clone().unsafe_builder(backend);
            builder.push(" Go to >> ");
            builder.push(&self.line_idx);
            builder.push_styled("|", ContentStyle::slowblink());
        }
        backend.set_style(reset_style);
    }

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        match gs.editor_area().right_top_corner(1, 50).into_iter().next() {
            Some(render_line) => {
                self.render_line = render_line;
                true
            }
            None => false,
        }
    }

    fn map_mouse(&mut self, event: MouseEvent, _: &mut Components) -> Status {
        if let MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. } = event {
            if !self.render_line.contains_position(row, column) {
                return Status::Finished;
            }
        };
        Status::Pending
    }

    fn render(&mut self, _gs: &mut GlobalState) {}
}

pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<bool>,
    pub state: usize,
    accent: ContentStyle,
    render_line: Line,
}

impl FindPopup {
    pub fn new(editor_area: Rect, accent: ContentStyle) -> Option<Self> {
        let render_line = editor_area.right_top_corner(1, 50).into_iter().next()?;
        let pattern = TextField::new(String::new(), Some(true));
        Some(Self { options: Vec::new(), pattern, state: 0, accent, render_line })
    }

    pub fn run_inplace(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let Some(mut popup) = FindPopup::new(*gs.editor_area(), gs.ui_theme.accent_style()) else {
            return;
        };
        let run_result = Popup::run(&mut popup, gs, workspace, tree, term);
        gs.log_if_error(run_result);
    }
}

impl Popup for FindPopup {
    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        let Components { gs, ws, tree, term } = components;

        if matches!(key.code, KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL)) {
            if let Some(mut popup) = ReplacePopup::from_search(
                self.pattern.text_take(),
                std::mem::take(&mut self.options),
                *gs.editor_area(),
                self.accent,
            ) {
                if let Err(error) = popup.run(gs, ws, tree, term) {
                    gs.error(error);
                };
            }
            return Status::Finished;
        }
        if Some(true) == self.pattern.map(&key, &mut gs.clipboard) {
            if let Some(editor) = ws.get_active() {
                self.options.clear();
                editor.find(self.pattern.as_str(), &mut self.options);
            }
            self.state = self.options.len().saturating_sub(1);
            self.force_render(gs);
            return Status::Pending;
        }
        let select_result = match key.code {
            KeyCode::Enter | KeyCode::Down => next_option(&self.options, &mut self.state),
            KeyCode::Up => prev_option(&self.options, &mut self.state),
            KeyCode::Esc | KeyCode::Left => return Status::Finished,
            KeyCode::Tab => {
                if let Some(editor) = ws.get_active() {
                    gs.insert_mode();
                    let options = editor.find_with_text(self.pattern.as_str());
                    let mut popup = PopupSelector::new(
                        options,
                        |((from, _), text), line, backend| line.render(&format!("({}) {text}", from.line + 1), backend),
                        go_to_select_command,
                        None,
                    );
                    if let Err(error) = popup.run(gs, ws, tree, term) {
                        gs.error(error);
                    };
                }
                return Status::Finished;
            }
            _ => return Status::Pending,
        };
        let Some(editor) = ws.get_active() else {
            return Status::Finished;
        };
        if let Some((from, to)) = select_result {
            editor.go_to_select(from, to);
            gs.backend.freeze();
            editor.render(gs);
            self.force_render(gs);
            gs.backend.unfreeze();
        }
        Status::Pending
    }

    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset_style = backend.get_style();
        backend.set_style(self.accent);
        {
            let mut builder = self.render_line.clone().unsafe_builder(backend);
            builder.push(" Found(");
            builder.push(&count_as_string(self.options.len()));
            builder.push(") >> ");
            self.pattern.insert_formatted_text(builder);
        }
        backend.set_style(reset_style);
    }

    fn render(&mut self, _gs: &mut GlobalState) {}

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        match gs.editor_area().right_top_corner(1, 50).into_iter().next() {
            Some(render_line) => {
                self.render_line = render_line;
                true
            }
            None => false,
        }
    }

    fn paste_passthrough(&mut self, clip: String, components: &mut Components) -> bool {
        self.pattern.paste_passthrough(clip);
        if let Some(editor) = components.ws.get_active() {
            self.options.clear();
            editor.find(self.pattern.as_str(), &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
        true
    }

    fn map_mouse(&mut self, event: MouseEvent, _: &mut Components) -> Status {
        if let MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. } = event {
            if !self.render_line.contains_position(row, column) {
                return Status::Finished;
            }
        };
        Status::Pending
    }
}

fn go_to_select_command(
    popup: &mut PopupSelector<((CursorPosition, CursorPosition), String)>,
    components: &mut Components,
) {
    let (from, to) = popup.options[popup.state.selected].0;
    let Some(editor) = components.ws.get_active() else {
        return;
    };
    editor.go_to_select(from, to);
}
