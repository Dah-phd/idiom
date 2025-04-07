use super::{
    utils::{into_message, next_option, prev_option},
    InplacePopup, PopupInterface, Status,
};
use crate::{
    embeded_term::EditorTerminal,
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{
        backend::{Backend, BackendProtocol, StyleExt},
        count_as_string,
        layout::{Line, Rect},
        TextField,
    },
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::ContentStyle;
use fuzzy_matcher::skim::SkimMatcherV2;

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
        _: crossterm::event::MouseEvent,
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

pub struct FindPopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<PopupMessage>,
    pub state: usize,
    accent: ContentStyle,
    render_line: Line,
}

impl FindPopup {
    pub fn new(editor_area: Rect, accent: ContentStyle) -> Option<Box<Self>> {
        let render_line = editor_area.right_top_corner(1, 50).into_iter().next()?;
        Some(Box::new(Self {
            options: Vec::new(),
            pattern: TextField::with_editor_access(String::new()),
            state: 0,
            accent,
            render_line,
        }))
    }
}

impl PopupInterface for FindPopup {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        if matches!(key.code, KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL)) {
            return PopupMessage::ClearEvent(IdiomEvent::FindToReplace(
                self.pattern.text.to_owned(),
                self.options.clone(),
            ));
        }
        if let Some(event) = self.pattern.map(key, clipboard) {
            return event;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Down => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc | KeyCode::Left => PopupMessage::Clear,
            KeyCode::Tab => PopupMessage::ClearEvent(IdiomEvent::FindSelector(self.pattern.text.to_owned())),
            _ => PopupMessage::None,
        }
    }

    fn render(&mut self, _screen: Rect, backend: &mut Backend) {
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

    fn fast_render(&mut self, screen: Rect, backend: &mut Backend) {
        self.render(screen, backend);
    }

    fn resize(&mut self, new_screen: Rect) -> PopupMessage {
        if new_screen.width < 100 {
            return PopupMessage::Clear;
        }
        let Some(render_line) = new_screen.right_top_corner(2, 50).into_iter().nth(1) else {
            return PopupMessage::Clear;
        };
        self.render_line = render_line;
        self.mark_as_updated();
        PopupMessage::None
    }

    fn component_access(&mut self, _gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree) {
        if let Some(editor) = ws.get_active() {
            self.options.clear();
            editor.find(self.pattern.text.as_str(), &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
    }

    fn mark_as_updated(&mut self) {}

    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        self.pattern.paste_passthrough(clip)
    }
}
