use super::{
    utils::{next_option, prev_option},
    InplacePopup, Status,
};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{GlobalState, IdiomEvent},
    render::{
        backend::{BackendProtocol, StyleExt},
        count_as_string,
        layout::{Line, Rect},
        TextField,
    },
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use crossterm::style::ContentStyle;

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
        let Some(mut popup) = GoToLinePopup::new(current_line, gs.editor_area, gs.theme.accent_style) else {
            return;
        };
        InplacePopup::run(&mut popup, gs, workspace, tree, term);
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

impl InplacePopup for GoToLinePopup {
    type R = ();

    fn map_keyboard(
        &mut self,
        key: KeyEvent,
        gs: &mut GlobalState,
        ws: &mut Workspace,
        _: &mut Tree,
        _: &mut EditorTerminal,
    ) -> Status<Self::R> {
        let result_idx = match key.code {
            KeyCode::Enter => return Status::Dropped,
            KeyCode::Char(ch) if ch.is_numeric() => {
                self.line_idx.push(ch);
                self.parse()
            }
            KeyCode::Backspace if self.line_idx.pop().is_some() => self.parse(),
            _ => return Status::Pending,
        };
        if let Some(line) = result_idx {
            let Some(editor) = ws.get_active() else {
                return Status::Dropped;
            };
            editor.go_to(line);
            gs.backend.freeze();
            editor.render(gs);
            self.render(gs);
            gs.backend.unfreeze();
        }
        Status::Pending
    }

    fn render(&mut self, gs: &mut GlobalState) {
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
        match gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            Some(render_line) => {
                self.render_line = render_line;
                true
            }
            None => false,
        }
    }

    fn map_mouse(
        &mut self,
        _: MouseEvent,
        _: &mut GlobalState,
        _: &mut Workspace,
        _: &mut Tree,
        _: &mut EditorTerminal,
    ) -> Status<Self::R> {
        Status::Pending
    }

    fn fast_render(&mut self, _gs: &mut GlobalState) {}

    fn mark_as_updated(&mut self) {}

    fn collect_update_status(&mut self) -> bool {
        false
    }
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
        Some(Self {
            options: Vec::new(),
            pattern: TextField::new(String::new(), Some(true)),
            state: 0,
            accent,
            render_line,
        })
    }

    pub fn run_inplace(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let Some(mut popup) = FindPopup::new(gs.editor_area, gs.theme.accent_style) else {
            return;
        };
        InplacePopup::run(&mut popup, gs, workspace, tree, term);
    }
}

impl InplacePopup for FindPopup {
    type R = ();

    fn map_keyboard(
        &mut self,
        key: KeyEvent,
        gs: &mut GlobalState,
        ws: &mut Workspace,
        _: &mut Tree,
        _: &mut EditorTerminal,
    ) -> Status<Self::R> {
        if matches!(key.code, KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL)) {
            gs.event.push(IdiomEvent::FindToReplace(self.pattern.text.to_owned(), self.options.clone()));
            return Status::Dropped;
        }
        if Some(true) == self.pattern.map(&key, &mut gs.clipboard) {
            if let Some(editor) = ws.get_active() {
                self.options.clear();
                editor.find(self.pattern.text.as_str(), &mut self.options);
            }
            self.state = self.options.len().saturating_sub(1);
            self.render(gs);
            return Status::Pending;
        }
        let select_result = match key.code {
            KeyCode::Enter | KeyCode::Down => next_option(&self.options, &mut self.state),
            KeyCode::Up => prev_option(&self.options, &mut self.state),
            KeyCode::Esc | KeyCode::Left => return Status::Dropped,
            KeyCode::Tab => {
                gs.event.push(IdiomEvent::FindSelector(self.pattern.text.to_owned()));
                return Status::Dropped;
            }
            _ => return Status::Pending,
        };
        let Some(editor) = ws.get_active() else {
            return Status::Dropped;
        };
        if let Some((from, to)) = select_result {
            editor.go_to_select(from, to);
            gs.backend.freeze();
            editor.render(gs);
            self.render(gs);
            gs.backend.unfreeze();
        }
        Status::Pending
    }

    fn render(&mut self, gs: &mut GlobalState) {
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

    fn fast_render(&mut self, _gs: &mut GlobalState) {}

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        match gs.editor_area.right_top_corner(1, 50).into_iter().next() {
            Some(render_line) => {
                self.render_line = render_line;
                true
            }
            None => false,
        }
    }

    fn paste_passthrough(&mut self, clip: String, _gs: &mut GlobalState) {
        self.pattern.paste_passthrough(clip);
    }

    fn map_mouse(
        &mut self,
        _: MouseEvent,
        _: &mut GlobalState,
        _: &mut Workspace,
        _: &mut Tree,
        _: &mut EditorTerminal,
    ) -> Status<Self::R> {
        Status::Pending
    }

    fn mark_as_updated(&mut self) {}

    fn collect_update_status(&mut self) -> bool {
        false
    }
}
