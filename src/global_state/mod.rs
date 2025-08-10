use std::error::Error;
mod clipboard;
mod controls;
mod draw;
mod events;
mod message;

use crate::{
    configs::{
        EditorConfigs, EditorKeyMap, FileType, GeneralKeyMap, KeyMap, TreeKeyMap, UITheme, EDITOR_CFG_FILE, KEY_MAP,
    },
    embeded_term::EditorTerminal,
    error::IdiomResult,
    ext_tui::CrossTerm,
    lsp::{LSPError, LSPResult},
    popups::{
        menu::{menu_context_editor_inplace, menu_context_tree_inplace},
        Popup,
    },
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
pub use clipboard::Clipboard;
pub use controls::Mode;
use crossterm::event::{KeyEvent, MouseEvent};
pub use events::IdiomEvent;
use idiom_tui::{
    layout::{Line, Rect},
    Backend,
};

use draw::Components;
use fuzzy_matcher::skim::SkimMatcherV2;
use message::Messages;

type KeyMapCallback = fn(&KeyEvent, &mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal) -> bool;
type MouseMapCallback = fn(MouseEvent, &mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal);
type PastePassthroughCallback = fn(&mut GlobalState, String, &mut Workspace, &mut EditorTerminal);
type DrawCallback = fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal);

pub struct GlobalState {
    pub backend: CrossTerm,
    pub theme: UITheme,
    pub matcher: SkimMatcherV2,
    pub event: Vec<IdiomEvent>,
    pub clipboard: Clipboard,
    pub screen_rect: Rect,
    pub tree_area: Rect,
    pub tab_area: Rect,
    pub editor_area: Rect,
    pub footer_line: Line,
    pub git_tui: Option<String>,
    messages: Messages,
    mode: Mode,
    tree_size: usize,
    key_mapper: KeyMapCallback,
    paste_passthrough: PastePassthroughCallback,
    mouse_mapper: MouseMapCallback,
    draw_callback: DrawCallback,
    components: Components,
}

impl GlobalState {
    pub fn new(screen_rect: Rect, backend: CrossTerm) -> Self {
        let mut messages = Messages::new();
        let theme = messages.unwrap_or_default(UITheme::new(), "Failed to load theme_ui.toml");
        Self {
            mode: Mode::default(),
            tree_size: std::cmp::max((15 * screen_rect.width) / 100, Mode::len()),
            key_mapper: controls::map_tree,
            paste_passthrough: controls::paste_passthrough_ignore,
            mouse_mapper: controls::mouse_handler,
            draw_callback: draw::full_rebuild,
            theme,
            backend,
            event: Vec::default(),
            clipboard: Clipboard::default(),
            screen_rect,
            git_tui: None,
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_line: Line::default(),
            matcher: SkimMatcherV2::default(),
            messages,
            components: Components::default(),
        }
    }

    pub fn get_configs(&mut self) -> EditorConfigs {
        let mut base_configs = self.unwrap_or_default(EditorConfigs::new(), EDITOR_CFG_FILE);
        self.git_tui = base_configs.git_tui.take();
        base_configs
    }

    pub fn get_key_maps(&mut self) -> (GeneralKeyMap, EditorKeyMap, TreeKeyMap) {
        self.unwrap_or_default(KeyMap::new(), KEY_MAP).unpack()
    }

    #[inline]
    pub fn map_key(
        &mut self,
        event: &KeyEvent,
        workspace: &mut Workspace,
        tree: &mut Tree,
        term: &mut EditorTerminal,
    ) -> bool {
        (self.key_mapper)(event, self, workspace, tree, term)
    }

    #[inline]
    pub fn map_mouse(
        &mut self,
        event: MouseEvent,
        tree: &mut Tree,
        workspace: &mut Workspace,
        term: &mut EditorTerminal,
    ) {
        (self.mouse_mapper)(event, self, workspace, tree, term)
    }

    pub fn passthrough_paste(&mut self, clip: String, workspace: &mut Workspace, term: &mut EditorTerminal) {
        (self.paste_passthrough)(self, clip, workspace, term);
    }

    pub fn select_mode(&mut self) {
        self.mode = Mode::Select;
        self.config_controls();
        if self.components.contains(Components::TREE) {
            let mut line = self.footer_line.clone();
            line.width = self.tree_size;
            Mode::render_select_mode(line, &mut self.backend);
        } else {
            self.draw_callback = draw::full_rebuild;
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.config_controls();
        if self.components.contains(Components::TREE) {
            let mut line = self.footer_line.clone();
            line.width = self.tree_size;
            Mode::render_insert_mode(line, &mut self.backend);
        } else {
            self.draw_callback = draw::full_rebuild;
        };
    }

    fn config_controls(&mut self) {
        if self.components.contains(Components::TERM) {
            self.key_mapper = controls::map_term;
            self.mouse_mapper = controls::mouse_term;
            self.paste_passthrough = controls::paste_passthrough_term;
            return;
        }
        match self.mode {
            Mode::Insert => {
                self.key_mapper = controls::map_editor;
                self.paste_passthrough = controls::paste_passthrough_editor;
            }
            Mode::Select => {
                self.key_mapper = controls::map_tree;
                self.paste_passthrough = controls::paste_passthrough_ignore;
            }
        }
        self.mouse_mapper = controls::mouse_handler;
    }

    #[inline]
    pub fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
    }

    #[inline]
    pub fn is_select(&self) -> bool {
        matches!(self.mode, Mode::Select)
    }

    pub fn toggle_tree(&mut self) {
        self.components.toggle(Components::TREE);
        self.draw_callback = draw::full_rebuild;
    }

    pub fn expand_tree_size(&mut self) {
        let max_size = self.screen_rect.width / 2;
        self.tree_size = std::cmp::min(max_size, self.tree_size + 1);
        self.draw_callback = draw::full_rebuild;
    }

    pub fn shrink_tree_size(&mut self) {
        let min_size = std::cmp::max((15 * self.screen_rect.width) / 100, Mode::len());
        self.tree_size = std::cmp::max(min_size, self.tree_size - 1);
        self.draw_callback = draw::full_rebuild;
    }

    pub fn toggle_terminal(&mut self, term: &mut EditorTerminal) {
        self.draw_callback = draw::full_rebuild;
        if self.components.contains(Components::TERM) {
            self.components.remove(Components::TERM);
            self.backend.hide_cursor();
        } else {
            self.components.insert(Components::TERM);
            term.activate(self.editor_area);
        }
        self.config_controls();
    }

    pub fn try_tree_event(&mut self, value: impl TryInto<IdiomEvent>) {
        if let Ok(event) = value.try_into() {
            self.event.push(event);
        }
    }

    // RENDER CONTROLS

    pub fn context_menu(&mut self, ws: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        let mut menu = match self.mode {
            Mode::Select => {
                let state = tree.get_state();
                let line = (state.selected - state.at_line) + 1;
                let char = self.tree_area.width / 2;
                let position = CursorPosition { line, char };
                let accent_style = self.theme.accent_style_reversed();
                menu_context_tree_inplace(position, self.screen_rect, accent_style)
            }
            Mode::Insert => {
                let Some(editor) = ws.get_active() else { return };
                let line = editor.cursor.line - editor.cursor.at_line;
                let char = editor.cursor.char + editor.line_number_offset + 1;
                let position = CursorPosition { line, char };
                let accent_style = self.theme.accent_style();
                menu_context_editor_inplace(position, self.editor_area, accent_style)
            }
        };
        if let Err(error) = menu.run(self, ws, tree, term) {
            self.error(error);
        };
    }

    #[inline(always)]
    pub fn backend(&mut self) -> &mut CrossTerm {
        &mut self.backend
    }

    #[inline]
    pub fn draw(&mut self, workspace: &mut Workspace, tree: &mut Tree, term: &mut EditorTerminal) {
        (self.draw_callback)(self, workspace, tree, term);
        self.backend.flush_buf();
    }

    pub fn render_stats(&mut self, len: usize, select_len: usize, cursor: CursorPosition) {
        let mut line = self.footer_line.clone();
        if self.components.contains(Components::TREE) || self.is_select() {
            line += self.tree_size;
        } else {
            line += Mode::len();
        }
        self.backend.set_style(self.theme.accent_style());
        let mut rev_builder = line.unsafe_builder_rev(&mut self.backend);
        if select_len != 0 {
            rev_builder.push(&format!("({select_len} selected) "));
        }
        rev_builder.push(&format!("  Doc Len {len}, Ln {}, Col {} ", cursor.line + 1, cursor.char + 1));
        self.messages.set_line(rev_builder.into_line());
        self.messages.fast_render(self.theme.accent_style(), &mut self.backend);
        self.backend.reset_style();
    }

    pub fn fast_render_message_with_preserved_cursor(&mut self) {
        if self.messages.should_render() {
            self.backend.save_cursor();
            self.messages.render(self.theme.accent_style(), &mut self.backend);
            self.backend.restore_cursor();
        }
    }

    pub fn render_footer_standalone(&mut self) {
        // reset expected line positions
        self.footer_line = self.screen_rect.clone().pop_line();
        let (mode_line, msg_line) = if self.components.contains(Components::TREE) || self.is_select() {
            self.footer_line.clone().split_rel(self.tree_size)
        } else {
            self.footer_line.clone().split_rel(Mode::len())
        };
        self.mode.render(mode_line, &mut self.backend);
        self.messages.set_line(msg_line);
        self.messages.render(self.theme.accent_style(), &mut self.backend);
    }

    #[inline]
    pub fn full_resize(&mut self, height: u16, width: u16) {
        let tree_rate = (self.tree_size * 100) / self.screen_rect.width;
        self.screen_rect = (width, height).into();
        self.tree_size = std::cmp::max((tree_rate * self.screen_rect.width) / 100, Mode::len());
        self.draw_callback = draw::full_rebuild;
    }

    #[inline]
    pub fn force_screen_rebuild(&mut self) {
        self.draw_callback = draw::full_rebuild;
    }

    pub fn force_area_calc(&mut self) {
        let mut screen = self.screen_rect;
        self.footer_line = screen.pop_line();
        let screen = if self.components.contains(Components::TREE) || self.is_select() {
            let (mut tree_area, tab_area) = screen.split_horizont_rel(self.tree_size);
            let _logo_line = tree_area.next_line();
            tree_area.right_border().left_border();
            self.tree_area = tree_area;
            tab_area
        } else {
            let (tree_area, tab_area) = screen.split_horizont_rel(0);
            self.tree_area = tree_area;
            tab_area
        };
        (self.tab_area, self.editor_area) = screen.split_vertical_rel(1);
    }

    pub fn clear_stats(&mut self) {
        let mut line = self.footer_line.clone();
        let accent_style = self.theme.accent_style();
        if self.components.contains(Components::TREE) || self.is_select() {
            line += self.tree_size;
        } else {
            line += Mode::len();
        }
        self.backend.set_style(accent_style);
        self.backend.go_to(line.row, line.col);
        self.backend.clear_to_eol();
        self.backend.reset_style();
        self.messages.render(accent_style, &mut self.backend);
    }

    // LOGGING

    #[inline]
    pub fn message(&mut self, msg: impl Into<String>) {
        self.messages.message(msg.into());
    }

    #[inline]
    pub fn error(&mut self, error: impl ToString) {
        self.messages.error(error.to_string());
    }

    #[inline]
    pub fn success(&mut self, msg: impl Into<String>) {
        self.messages.success(msg.into());
    }

    /// unwrap or default with logged error
    #[inline]
    pub fn unwrap_or_default<T: Default, E: Error>(&mut self, result: Result<T, E>, prefix: &str) -> T {
        self.messages.unwrap_or_default(result, prefix)
    }

    /// logs IdiomError and drops the result
    #[inline]
    pub fn log_if_error<Any>(&mut self, result: IdiomResult<Any>) {
        if let Err(error) = result {
            self.error(error);
        }
    }

    /// unwrap LSP error and check status
    #[inline]
    pub fn log_if_lsp_error(&mut self, result: LSPResult<()>, file_type: FileType) {
        if let Err(err) = result {
            self.send_error(err, file_type);
        }
    }

    /// handle LSP error types
    #[inline]
    pub fn send_error(&mut self, err: LSPError, file_type: FileType) {
        match err {
            LSPError::Null => (),
            LSPError::InternalError(message) => {
                self.messages.error(message);
                self.event.push(IdiomEvent::CheckLSP(file_type));
            }
            _ => self.error(err),
        }
    }

    pub async fn handle_events(&mut self, tree: &mut Tree, ws: &mut Workspace, term: &mut EditorTerminal) {
        tree.sync(self);
        while let Some(event) = self.event.pop() {
            event.handle(self, ws, tree, term).await
        }
    }
}

#[cfg(test)]
mod tests;
