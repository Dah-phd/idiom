use std::error::Error;
mod clipboard;
mod controls;
mod draw;
mod events;
mod message;

use crate::{
    configs::{FileType, UITheme},
    error::IdiomResult,
    lsp::{LSPError, LSPResult},
    popups::{self, PopupInterface},
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
    },
    runner::EditorTerminal,
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
pub use clipboard::Clipboard;
pub use controls::{Mode, PopupMessage};
use crossterm::event::{KeyEvent, MouseEvent};
pub use events::IdiomEvent;

use draw::Components;
use fuzzy_matcher::skim::SkimMatcherV2;
use message::Messages;

type KeyMapCallback = fn(&mut GlobalState, &KeyEvent, &mut Workspace, &mut Tree, &mut EditorTerminal) -> bool;
type MouseMapCallback = fn(&mut GlobalState, MouseEvent, &mut Tree, &mut Workspace);
type DrawCallback = fn(&mut GlobalState, &mut Workspace, &mut Tree, &mut EditorTerminal) -> std::io::Result<()>;

pub struct GlobalState {
    mode: Mode,
    tree_size: usize,
    key_mapper: KeyMapCallback,
    mouse_mapper: MouseMapCallback,
    draw_callback: DrawCallback,
    pub theme: UITheme,
    pub writer: Backend,
    pub popup: Box<dyn PopupInterface>,
    pub event: Vec<IdiomEvent>,
    pub clipboard: Clipboard,
    pub exit: bool,
    pub screen_rect: Rect,
    pub tree_area: Rect,
    pub tab_area: Rect,
    pub editor_area: Rect,
    pub footer_area: Rect,
    pub matcher: SkimMatcherV2,
    messages: Messages,
    components: Components,
}

impl GlobalState {
    pub fn new(backend: Backend) -> std::io::Result<Self> {
        let mut messages = Messages::new();
        let theme = messages.unwrap_or_default(UITheme::new(), "Failed to load theme_ui.toml");
        Backend::screen().map(|screen_rect| Self {
            mode: Mode::default(),
            tree_size: 15,
            key_mapper: controls::map_tree,
            mouse_mapper: controls::mouse_handler,
            draw_callback: draw::full_rebuild,
            theme,
            writer: backend,
            popup: popups::placeholder(),
            event: Vec::default(),
            clipboard: Clipboard::default(),
            exit: false,
            screen_rect,
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_area: Rect::default(),
            matcher: SkimMatcherV2::default(),
            messages,
            components: Components::default(),
        })
    }

    #[inline(always)]
    pub fn backend(&mut self) -> &mut Backend {
        &mut self.writer
    }

    #[inline]
    pub fn draw(
        &mut self,
        workspace: &mut Workspace,
        tree: &mut Tree,
        term: &mut EditorTerminal,
    ) -> std::io::Result<()> {
        (self.draw_callback)(self, workspace, tree, term)
    }

    pub fn render_stats(&mut self, len: usize, select_len: usize, cursor: CursorPosition) {
        if let Some(mut line) = self.footer_area.get_line(0) {
            line += Mode::len();
            self.writer.set_style(self.theme.accent_style);
            let mut rev_builder = line.unsafe_builder_rev(&mut self.writer);
            if select_len != 0 {
                rev_builder.push(&format!(" ({select_len} selected)"));
            }
            rev_builder.push(&format!("  Doc Len {len}, Ln {}, Col {}", cursor.line + 1, cursor.char + 1));
            self.messages.set_line(rev_builder.into_line());
            self.messages.fast_render(self.theme.accent_style, &mut self.writer);
            self.writer.reset_style();
        }
    }

    pub fn clear_stats(&mut self) {
        if let Some(mut line) = self.footer_area.get_line(0) {
            let accent_style = self.theme.accent_style;
            line += Mode::len();
            self.writer.set_style(accent_style);
            self.writer.go_to(line.row, line.col);
            self.writer.clear_to_eol();
            self.writer.reset_style();
            self.messages.render(accent_style, &mut self.writer);
        }
    }

    #[inline]
    pub fn map_key(
        &mut self,
        event: &KeyEvent,
        workspace: &mut Workspace,
        tree: &mut Tree,
        tmux: &mut EditorTerminal,
    ) -> bool {
        (self.key_mapper)(self, event, workspace, tree, tmux)
    }

    #[inline]
    pub fn map_mouse(&mut self, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
        (self.mouse_mapper)(self, event, tree, workspace)
    }

    fn config_controls(&mut self) {
        if self.components.contains(Components::POPUP) {
            self.key_mapper = controls::map_popup;
            self.mouse_mapper = controls::disable_mouse;
            return;
        }
        if self.components.contains(Components::TERM) {
            self.key_mapper = controls::map_term;
            self.mouse_mapper = controls::disable_mouse;
            return;
        }
        match self.mode {
            Mode::Insert => {
                self.key_mapper = controls::map_editor;
                self.mouse_mapper = controls::mouse_handler;
            }
            Mode::Select => {
                self.key_mapper = controls::map_tree;
                self.mouse_mapper = controls::mouse_handler;
            }
        }
    }

    pub fn select_mode(&mut self) {
        self.mode = Mode::Select;
        self.config_controls();
        if !self.components.contains(Components::TREE) {
            self.draw_callback = draw::full_rebuild;
        };
        if let Some(line) = self.footer_area.get_line(0) {
            Mode::render_select_mode(line, self.theme.accent_style, &mut self.writer);
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.config_controls();
        if !self.components.contains(Components::TREE) {
            self.draw_callback = draw::full_rebuild;
        };
        if let Some(line) = self.footer_area.get_line(0) {
            Mode::render_insert_mode(line, self.theme.accent_style, &mut self.writer);
        };
    }

    #[inline]
    pub fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
    }

    #[inline]
    pub fn has_popup(&self) -> bool {
        self.components.contains(Components::POPUP)
    }

    #[inline]
    pub fn popup_render(&mut self) {
        // popups do not mutate during render
        let gs = unsafe { &mut *(self as *mut GlobalState) };
        self.popup.fast_render(gs);
    }

    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        self.components.insert(Components::POPUP);
        self.config_controls();
        self.draw_callback = draw::full_rebuild;
        self.mouse_mapper = controls::mouse_popup_handler;
        self.popup = popup;
    }

    pub fn clear_popup(&mut self) {
        self.components.remove(Components::POPUP);
        self.config_controls();
        self.draw_callback = draw::full_rebuild;
        self.editor_area.clear(&mut self.writer);
        self.tree_area.clear(&mut self.writer);
        self.popup = popups::placeholder();
    }

    pub fn toggle_tree(&mut self) {
        self.components.toggle(Components::TREE);
        self.draw_callback = draw::full_rebuild;
    }

    pub fn expand_tree_size(&mut self) {
        self.tree_size = std::cmp::min(75, self.tree_size + 1);

        self.draw_callback = draw::full_rebuild;
    }

    pub fn shrink_tree_size(&mut self) {
        self.tree_size = std::cmp::max(15, self.tree_size - 1);
        self.draw_callback = draw::full_rebuild;
    }

    pub fn toggle_terminal(&mut self, runner: &mut EditorTerminal) {
        self.draw_callback = draw::full_rebuild;
        if self.components.contains(Components::TERM) {
            self.components.remove(Components::TERM);
        } else {
            self.components.insert(Components::TERM);
            runner.activate();
        }
        self.config_controls();
    }

    pub fn map_popup_if_exists(&mut self, key: &KeyEvent) -> bool {
        match self.popup.map(key, &mut self.clipboard, &self.matcher) {
            PopupMessage::Clear => {
                self.clear_popup();
            }
            PopupMessage::None => {}
            PopupMessage::Event(event) => {
                self.event.push(event);
            }
        }
        true
    }

    pub fn try_tree_event(&mut self, value: impl TryInto<IdiomEvent>) {
        if let Ok(event) = value.try_into() {
            self.event.push(event);
        }
    }

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

    #[inline]
    pub fn full_resize(&mut self, height: u16, width: u16) {
        self.screen_rect = (width, height).into();
        self.draw_callback = draw::full_rebuild;
        self.event.push(IdiomEvent::Resize);
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

    pub async fn exchange_should_exit(&mut self, tree: &mut Tree, ws: &mut Workspace) -> bool {
        tree.sync(self);
        while let Some(event) = self.event.pop() {
            event.handle(self, ws, tree).await
        }
        self.exit
    }
}

#[cfg(test)]
mod tests;
