use crate::{
    embeded_term::EditorTerminal,
    ext_tui::{text_field::map_key, StyleExt},
    global_state::GlobalState,
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::ContentStyle,
};
use idiom_tui::{
    layout::Rect,
    text_field::{Status as InputStatus, TextField},
    Backend,
};

use super::{
    utils::{count_as_string, next_option, prev_option},
    Components, Popup, Status,
};

pub struct ReplacePopup {
    pub options: Vec<(CursorPosition, CursorPosition)>,
    pub pattern: TextField,
    pub new_text: TextField,
    pub on_text: bool,
    pub state: usize,
    accent: ContentStyle,
    rect: Rect,
}

impl ReplacePopup {
    pub fn run_inplace(gs: &mut GlobalState, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let rect = gs.editor_area().right_top_corner(2, 50);
        if rect.height < 2 {
            return;
        }

        let mut popup: ReplacePopup = Self {
            rect,
            accent: gs.ui_theme.accent_style(),
            on_text: false,
            options: Vec::new(),
            pattern: TextField::default(),
            new_text: TextField::default(),
            state: usize::default(),
        };

        if let Err(error) = popup.run(gs, workspace, tree, term) {
            gs.error(error);
        }
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
            pattern: TextField::new(pattern),
            new_text: TextField::default(),
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

impl Popup for ReplacePopup {
    fn map_keyboard(&mut self, key: KeyEvent, components: &mut Components) -> Status {
        let Components { gs, ws, .. } = components;

        let Some(editor) = ws.get_active() else {
            return Status::Finished;
        };

        match key.code {
            KeyCode::Char('h' | 'H') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.options.is_empty() {
                    return Status::Pending;
                }
                let (from, to) = self.drain_next();
                gs.backend.freeze();
                editor.replace_select(from, to, self.new_text.as_str());
                if let Some((from, to)) = self.get_state() {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                } else {
                    return Status::Finished;
                }
                self.force_render(gs);
                gs.backend.unfreeze();
            }
            KeyCode::Char('a' | 'A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.options.is_empty() {
                    let clip = self.new_text.text_take();
                    let ranges = std::mem::take(&mut self.options.clone());
                    editor.mass_replace(ranges, clip);
                }
                return Status::Finished;
            }
            KeyCode::Tab => {
                self.on_text = !self.on_text;
                self.force_render(gs);
            }
            KeyCode::Down | KeyCode::Enter => {
                if let Some((from, to)) = next_option(&self.options, &mut self.state) {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                    self.force_render(gs);
                }
            }
            KeyCode::Up => {
                if let Some((from, to)) = prev_option(&self.options, &mut self.state) {
                    editor.go_to_select(from, to);
                    editor.render(gs);
                    self.force_render(gs);
                }
            }
            _ => {
                let result = match self.on_text {
                    true => map_key(&mut self.new_text, key, &mut gs.clipboard),
                    false => map_key(&mut self.pattern, key, &mut gs.clipboard),
                };
                match result {
                    Some(InputStatus::Skipped) | None => {}
                    Some(InputStatus::Updated) => {
                        self.options.clear();
                        editor.find(self.pattern.as_str(), &mut self.options);
                        self.state = self.options.len().saturating_sub(1);
                        self.force_render(gs);
                    }
                    Some(InputStatus::UpdatedCursor) => self.force_render(gs),
                }
            }
        }
        Status::Pending
    }

    fn map_mouse(&mut self, _: crossterm::event::MouseEvent, _: &mut Components) -> Status {
        Status::Pending
    }

    fn force_render(&mut self, gs: &mut GlobalState) {
        let backend = &mut gs.backend;
        let reset = backend.get_style();
        backend.set_style(self.accent);
        let mut lines = self.rect.into_iter();
        if let Some(line) = lines.next() {
            let mut find_builder = line.unsafe_builder(backend);
            find_builder.push(count_as_string(&self.options).as_str());
            find_builder.push(" > ");
            match self.on_text {
                false => self.pattern.insert_formatted_text(
                    find_builder,
                    ContentStyle::reversed(),
                    gs.ui_theme.accent_select_style(),
                ),
                true => {
                    find_builder.push(self.pattern.as_str());
                }
            }
        };
        if let Some(line) = lines.next() {
            let mut repl_builder = line.unsafe_builder(backend);
            repl_builder.push("Rep > ");
            match self.on_text {
                false => {
                    repl_builder.push(self.new_text.as_str());
                }
                true => self.new_text.insert_formatted_text(
                    repl_builder,
                    ContentStyle::reversed(),
                    gs.ui_theme.accent_select_style(),
                ),
            }
        }
        backend.set_style(reset);
    }

    fn render(&mut self, _gs: &mut GlobalState) {}

    fn resize_success(&mut self, gs: &mut GlobalState) -> bool {
        let rect = gs.editor_area().right_top_corner(2, 50);
        if rect.height < 2 {
            return false;
        }
        self.rect = rect;
        true
    }

    fn paste_passthrough(&mut self, clip: String, _: &mut Components) -> bool {
        match self.on_text {
            true => self.new_text.paste_passthrough(clip),
            false => self.pattern.paste_passthrough(clip),
        };
        true
    }
}
