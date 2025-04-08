use crate::{
    embeded_term::EditorTerminal,
    global_state::{Clipboard, GlobalState, IdiomEvent, PopupMessage},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
        TextField,
    },
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::ContentStyle,
};
use fuzzy_matcher::skim::SkimMatcherV2;

use super::{
    utils::{count_as_string, into_message, next_option, prev_option},
    Components, InplacePopup, PopupInterface, Status,
};

pub struct ReplacePopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<PopupMessage>,
    pub new_text: TextField<PopupMessage>,
    pub on_text: bool,
    pub state: usize,
    accent: ContentStyle,
    rect: Rect,
}

impl ReplacePopup {
    pub fn new(editor_area: Rect, accent: ContentStyle) -> Option<Box<Self>> {
        let rect = editor_area.right_top_corner(2, 50);
        if rect.height < 2 {
            return None;
        }
        Some(Box::new(Self {
            rect,
            accent,
            on_text: false,
            options: Vec::new(),
            pattern: TextField::with_editor_access(String::new()),
            new_text: TextField::with_editor_access(String::new()),
            state: usize::default(),
        }))
    }

    pub fn from_search(
        pattern: String,
        options: Vec<(CursorPosition, CursorPosition)>,
        editor_area: Rect,
        accent: ContentStyle,
    ) -> Option<Box<Self>> {
        let rect = editor_area.right_top_corner(2, 50);
        if rect.height < 2 {
            return None;
        }
        Some(Box::new(Self {
            on_text: true,
            pattern: TextField::with_editor_access(pattern),
            new_text: TextField::with_editor_access(String::new()),
            options,
            state: 0,
            accent,
            rect,
        }))
    }

    fn drain_next(&mut self) -> (CursorPosition, CursorPosition) {
        let position = self.options.remove(self.state);
        if self.state >= self.options.len() {
            self.state = 0;
        }
        position
    }

    fn get_state(&self) -> Option<(CursorPosition, CursorPosition)> {
        self.options.get(self.state).cloned()
    }
}

impl PopupInterface for ReplacePopup {
    fn key_map(&mut self, key: &KeyEvent, clipboard: &mut Clipboard, _: &SkimMatcherV2) -> PopupMessage {
        match key.code {
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::None;
                }
                IdiomEvent::ReplaceNextSelect {
                    new_text: self.new_text.text.to_owned(),
                    select: self.drain_next(),
                    next_select: self.get_state(),
                }
                .into()
            }
            KeyCode::Char('a' | 'A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return PopupMessage::Clear;
                }
                PopupMessage::ClearEvent(IdiomEvent::ReplaceAll(self.new_text.text.to_owned(), self.options.clone()))
            }
            KeyCode::Tab => {
                self.on_text = !self.on_text;
                PopupMessage::None
            }
            KeyCode::Down | KeyCode::Enter => into_message(next_option(&self.options, &mut self.state)),
            KeyCode::Up => into_message(prev_option(&self.options, &mut self.state)),
            KeyCode::Esc => PopupMessage::Clear,
            _ => match self.on_text {
                true => self.new_text.map(key, clipboard),
                false => self.pattern.map(key, clipboard),
            }
            .unwrap_or_default(),
        }
    }

    fn render(&mut self, _screen: Rect, backend: &mut Backend) {
        let reset = backend.get_style();
        backend.set_style(self.accent);
        let mut lines = self.rect.into_iter();
        if let Some(line) = lines.next() {
            let mut find_builder = line.unsafe_builder(backend);
            find_builder.push(count_as_string(&self.options).as_str());
            find_builder.push(" > ");
            match self.on_text {
                false => self.pattern.insert_formatted_text(find_builder),
                true => {
                    find_builder.push(&self.pattern.text);
                }
            }
        };
        if let Some(line) = lines.next() {
            let mut repl_builder = line.unsafe_builder(backend);
            repl_builder.push("Rep > ");
            match self.on_text {
                false => {
                    repl_builder.push(&self.new_text.text);
                }
                true => self.new_text.insert_formatted_text(repl_builder),
            }
        }
        backend.set_style(reset);
    }

    fn fast_render(&mut self, screen: Rect, backend: &mut Backend) {
        self.render(screen, backend);
    }

    fn resize(&mut self, mut new_screen: Rect) -> PopupMessage {
        if new_screen.width < 100 || new_screen.height < 3 {
            return PopupMessage::Clear;
        }
        let _skip_tab_bar = new_screen.next_line();
        self.rect = new_screen.right_top_corner(2, 50);
        self.mark_as_updated();
        PopupMessage::None
    }

    fn component_access(&mut self, gs: &mut GlobalState, ws: &mut Workspace, _tree: &mut Tree) {
        if let Some(editor) = ws.get_active() {
            self.options.clear();
            editor.find(&self.pattern.text, &mut self.options);
        }
        self.state = self.options.len().saturating_sub(1);
    }

    fn paste_passthrough(&mut self, clip: String, _: &SkimMatcherV2) -> PopupMessage {
        match self.on_text {
            true => self.new_text.paste_passthrough(clip),
            false => self.pattern.paste_passthrough(clip),
        }
    }

    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn mark_as_updated(&mut self) {}
}

pub struct ReplacePopupX {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField<bool>,
    pub new_text: TextField<bool>,
    pub on_text: bool,
    pub state: usize,
    accent: ContentStyle,
    rect: Rect,
}

impl ReplacePopupX {
    pub fn run_inplace(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let rect = gs.editor_area.right_top_corner(2, 50);
        if rect.height < 2 {
            return;
        }
        Self {
            rect,
            accent: gs.theme.accent_style,
            on_text: false,
            options: Vec::new(),
            pattern: TextField::new(String::new(), Some(true)),
            new_text: TextField::new(String::new(), Some(true)),
            state: usize::default(),
        }
        .run(gs, workspace, tree, term);
    }

    pub fn from_search(
        pattern: String,
        options: Vec<(CursorPosition, CursorPosition)>,
        editor_area: Rect,
        accent: ContentStyle,
    ) -> Option<Self> {
        let rect = editor_area.right_top_corner(2, 50);
        if rect.height < 2 {
            return None;
        }
        Some(Self {
            on_text: true,
            pattern: TextField::new(pattern, Some(true)),
            new_text: TextField::new(String::new(), Some(true)),
            options,
            state: 0,
            accent,
            rect,
        })
    }

    fn drain_next(&mut self) -> (CursorPosition, CursorPosition) {
        let position = self.options.remove(self.state);
        if self.state >= self.options.len() {
            self.state = 0;
        }
        position
    }

    fn get_state(&self) -> Option<(CursorPosition, CursorPosition)> {
        self.options.get(self.state).cloned()
    }
}

impl InplacePopup for ReplacePopupX {
    type R = ();

    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status<Self::R> {
        let Components { gs, ws, .. } = components;

        let Some(editor) = ws.get_active() else {
            return Status::Dropped;
        };

        match key.code {
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return Status::Pending;
                }
                let (from, to) = self.drain_next();
                gs.backend.freeze();
                editor.replace_select(from, to, &self.new_text.text);
                if let Some((from, to)) = self.get_state() {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                } else {
                    editor.render(gs);
                }
                self.render(gs);
                gs.backend.unfreeze();
            }
            KeyCode::Char('a' | 'A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    let clip = std::mem::take(&mut self.new_text.text);
                    let ranges = std::mem::take(&mut self.options.clone());
                    editor.mass_replace(ranges, clip);
                }
                return Status::Dropped;
            }
            KeyCode::Tab => {
                self.on_text = !self.on_text;
                self.render(gs);
            }
            KeyCode::Down | KeyCode::Enter => {
                if let Some((from, to)) = next_option(&self.options, &mut self.state) {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                    self.render(gs);
                }
            }
            KeyCode::Up => {
                if let Some((from, to)) = prev_option(&self.options, &mut self.state) {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                    self.render(gs);
                }
            }
            _ => {
                if Some(true)
                    == match self.on_text {
                        true => self.new_text.map(&key, &mut gs.clipboard),
                        false => self.pattern.map(&key, &mut gs.clipboard),
                    }
                {
                    self.options.clear();
                    editor.find(&self.pattern.text, &mut self.options);
                    self.state = self.options.len().saturating_sub(1);
                }
                self.render(gs);
            }
        }
        Status::Pending
    }

    fn map_mouse(&mut self, _: crossterm::event::MouseEvent, _: &mut Components) -> Status<Self::R> {
        Status::Pending
    }

    fn render(&mut self, gs: &mut GlobalState) {
        let backend = gs.backend();
        let reset = backend.get_style();
        backend.set_style(self.accent);
        let mut lines = self.rect.into_iter();
        if let Some(line) = lines.next() {
            let mut find_builder = line.unsafe_builder(backend);
            find_builder.push(count_as_string(&self.options).as_str());
            find_builder.push(" > ");
            match self.on_text {
                false => self.pattern.insert_formatted_text(find_builder),
                true => {
                    find_builder.push(&self.pattern.text);
                }
            }
        };
        if let Some(line) = lines.next() {
            let mut repl_builder = line.unsafe_builder(backend);
            repl_builder.push("Rep > ");
            match self.on_text {
                false => {
                    repl_builder.push(&self.new_text.text);
                }
                true => self.new_text.insert_formatted_text(repl_builder),
            }
        }
        backend.set_style(reset);
    }

    fn fast_render(&mut self, gs: &mut GlobalState) {}

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        let rect = gs.editor_area.right_top_corner(2, 50);
        if rect.height < 2 {
            return false;
        }
        self.rect = rect;
        self.mark_as_updated();
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        match self.on_text {
            true => self.new_text.paste_passthrough(clip),
            false => self.pattern.paste_passthrough(clip),
        };
        true
    }

    fn collect_update_status(&mut self) -> bool {
        true
    }

    fn mark_as_updated(&mut self) {}
}
