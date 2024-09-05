use std::error::Error;
mod clipboard;
mod controls;
mod draw;
mod events;
mod message;

use crate::{
    configs::{FileType, UITheme},
    lsp::{LSPError, LSPResult},
    popups::{
        placeholder, popup_replace::ReplacePopup, popup_tree_search::ActiveFileSearch, popups_editor::selector_ranges,
        PopupInterface,
    },
    render::{
        backend::{color, Backend, BackendProtocol, Style},
        layout::{Line, Rect, DOUBLE_BORDERS},
    },
    runner::EditorTerminal,
    tree::Tree,
    workspace::{CursorPosition, Workspace},
};
pub use clipboard::Clipboard;
use controls::map_term;
use crossterm::event::{KeyEvent, MouseEvent};
pub use events::IdiomEvent;

use self::{draw::Components, message::Messages};

const INSERT_SPAN: &str = "  --INSERT--   ";
const SELECT_SPAN: &str = "  --SELECT--   ";

#[derive(Default, Clone)]
pub enum PopupMessage {
    #[default]
    None,
    Tree(IdiomEvent),
    Clear,
}

#[derive(Default)]
enum Mode {
    #[default]
    Select,
    Insert,
}

impl Mode {
    #[inline]
    fn render(&self, line: Line, accent_style: Style, backend: &mut Backend) {
        match self {
            Self::Insert => Self::render_insert_mode(line, accent_style, backend),
            Self::Select => Self::render_select_mode(line, accent_style, backend),
        };
    }

    #[inline]
    fn render_select_mode(mut line: Line, mut accent_style: Style, backend: &mut Backend) {
        line.width = std::cmp::min(SELECT_SPAN.len(), line.width);
        accent_style.add_bold();
        accent_style.set_fg(Some(color::cyan()));
        line.render_styled(SELECT_SPAN, accent_style, backend);
    }

    #[inline]
    fn render_insert_mode(mut line: Line, mut accent_style: Style, backend: &mut Backend) {
        line.width = std::cmp::min(INSERT_SPAN.len(), line.width);
        accent_style.add_bold();
        accent_style.set_fg(Some(color::rgb(255, 0, 0)));
        line.render_styled(INSERT_SPAN, accent_style, backend);
    }
}

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
    messages: Messages,
    components: Components,
}

impl GlobalState {
    pub fn new(backend: Backend) -> std::io::Result<Self> {
        let mut messages = Messages::new();
        let theme = messages.unwrap_or_default(UITheme::new(), "Failed to load theme_ui.json");
        Backend::screen().map(|screen_rect| Self {
            mode: Mode::default(),
            tree_size: 15,
            key_mapper: controls::map_tree,
            mouse_mapper: controls::mouse_handler,
            draw_callback: draw::full_rebuild,
            theme,
            writer: backend,
            popup: placeholder(),
            event: Vec::default(),
            clipboard: Clipboard::default(),
            exit: false,
            screen_rect,
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_area: Rect::default(),
            messages,
            components: Components::default(),
        })
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
            line += INSERT_SPAN.len();
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

    pub fn select_mode(&mut self) {
        self.mode = Mode::Select;
        if !self.components.contains(Components::POPUP) {
            self.key_mapper = controls::map_tree;
        }
        if !self.components.contains(Components::TREE) {
            self.draw_callback = draw::full_rebuild;
        };
        if let Some(line) = self.footer_area.get_line(0) {
            Mode::render_select_mode(line, self.theme.accent_style, &mut self.writer);
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        if !self.components.contains(Components::POPUP) {
            self.key_mapper = controls::map_editor;
        }
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
    pub fn render_popup(&mut self) {
        // popups do not mutate during render
        let gs = unsafe { &mut *(self as *mut GlobalState) };
        self.popup.fast_render(gs);
    }

    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        self.components.insert(Components::POPUP);
        self.key_mapper = controls::map_popup;
        self.draw_callback = draw::full_rebuild;
        self.mouse_mapper = controls::disable_mouse;
        self.popup = popup;
    }

    pub fn clear_popup(&mut self) {
        match self.mode {
            Mode::Select => {
                self.key_mapper = controls::map_tree;
            }
            Mode::Insert => {
                self.key_mapper = controls::map_editor;
            }
        }
        self.components.remove(Components::POPUP);
        self.draw_callback = draw::full_rebuild;
        self.mouse_mapper = controls::mouse_handler;
        self.editor_area.clear(&mut self.writer);
        self.tree_area.clear(&mut self.writer);
        self.popup = placeholder();
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
            match self.mode {
                Mode::Select => {
                    self.key_mapper = controls::map_tree;
                }
                Mode::Insert => {
                    self.key_mapper = controls::map_editor;
                }
            }
            self.mouse_mapper = controls::mouse_handler;
        } else {
            self.components.insert(Components::TERM);
            runner.activate();
            self.key_mapper = map_term;
            self.mouse_mapper = controls::disable_mouse;
        }
    }

    pub fn map_popup_if_exists(&mut self, key: &KeyEvent) -> bool {
        match self.popup.map(key, &mut self.clipboard) {
            PopupMessage::Clear => {
                self.clear_popup();
            }
            PopupMessage::None => {}
            PopupMessage::Tree(event) => {
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
    pub fn error(&mut self, msg: impl Into<String>) {
        self.messages.error(msg.into());
    }

    #[inline]
    pub fn success(&mut self, msg: impl Into<String>) {
        self.messages.success(msg.into());
    }

    #[inline]
    pub fn full_resize(&mut self, height: u16, width: u16, workspace: &mut Workspace) {
        self.screen_rect = (width, height).into();
        self.draw_callback = draw::full_rebuild;
        workspace.resize_all(self.editor_area.width, self.editor_area.height as usize);
    }

    pub fn recalc_draw_size(&mut self) {
        self.tree_area = self.screen_rect;
        self.footer_area = self.tree_area.splitoff_rows(1);
        if let Some(mut line) = self.footer_area.get_line(0) {
            line += SELECT_SPAN.len();
            self.messages.set_line(line);
        };
        if matches!(self.mode, Mode::Select) || self.components.contains(Components::TREE) {
            self.tab_area = self.tree_area.keep_col((self.tree_size * self.screen_rect.width) / 100);
            self.tree_area.top_border().right_border().draw_borders(
                Some(DOUBLE_BORDERS),
                Some(color::dark_grey()),
                &mut self.writer,
            );
        } else {
            self.tab_area = self.tree_area.keep_col(0);
        };
        self.editor_area = self.tab_area.keep_rows(1);
    }

    /// unwrap or default with logged error
    #[inline]
    pub fn unwrap_or_default<T: Default, E: Error>(&mut self, result: Result<T, E>, prefix: &str) -> T {
        self.messages.unwrap_or_default(result, prefix)
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
            _ => self.error(err.to_string()),
        }
    }

    pub async fn exchange_should_exit(&mut self, tree: &mut Tree, ws: &mut Workspace) -> bool {
        tree.finish_sync(self);
        for event in std::mem::take(&mut self.event) {
            match event {
                IdiomEvent::PopupAccess => {
                    self.popup.component_access(ws, tree);
                }
                IdiomEvent::SearchFiles(pattern) => {
                    if pattern.len() > 1 {
                        let mut new_popup = ActiveFileSearch::new(pattern);
                        new_popup.component_access(ws, tree);
                        self.popup(new_popup);
                    } else {
                        self.popup(ActiveFileSearch::new(pattern));
                    }
                }
                IdiomEvent::Open(path) => {
                    tree.select_by_path(&path);
                    self.clear_popup();
                    if path.is_dir() {
                        self.select_mode();
                    } else {
                        match ws.new_from(path, self).await {
                            Ok(..) => self.insert_mode(),
                            Err(error) => self.error(error.to_string()),
                        }
                    }
                }
                IdiomEvent::OpenAtLine(path, line) => {
                    tree.select_by_path(&path);
                    self.clear_popup();
                    match ws.new_at_line(path, line, self).await {
                        Ok(..) => self.insert_mode(),
                        Err(error) => self.error(error.to_string()),
                    }
                }
                IdiomEvent::OpenAtSelect(path, select) => {
                    tree.select_by_path(&path);
                    match ws.new_from(path, self).await {
                        Ok(..) => self.insert_mode(),
                        Err(error) => self.error(error.to_string()),
                    }
                    self.event.push(IdiomEvent::GoToSelect { select, clear_popup: true });
                }
                IdiomEvent::SelectPath(path) => {
                    tree.select_by_path(&path);
                }
                IdiomEvent::CreateFileOrFolder(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder(name) {
                        if !new_path.is_dir() {
                            match ws.new_at_line(new_path, 0, self).await {
                                Ok(..) => {
                                    self.insert_mode();
                                    if let Some(editor) = ws.get_active() {
                                        editor.update_status.deny();
                                    }
                                }
                                Err(error) => self.error(error.to_string()),
                            };
                        }
                    }
                    self.clear_popup();
                }
                IdiomEvent::CreateFileOrFolderBase(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder_base(name) {
                        if !new_path.is_dir() {
                            match ws.new_at_line(new_path, 0, self).await {
                                Ok(..) => {
                                    self.insert_mode();
                                    if let Some(editor) = ws.get_active() {
                                        editor.update_status.deny();
                                    }
                                }
                                Err(error) => self.error(error.to_string()),
                            };
                        }
                    }
                    self.clear_popup();
                }
                IdiomEvent::RenameFile(name) => {
                    if let Some(result) = tree.rename_path(name) {
                        match result {
                            Ok((old, new_path)) => ws.rename_editors(old, new_path, self),
                            Err(err) => self.messages.error(err.to_string()),
                        }
                    };
                    self.clear_popup();
                }
                IdiomEvent::RegisterLSP(lsp) => {
                    tree.register_lsp(lsp);
                }
                IdiomEvent::AutoComplete(completion) => {
                    if let Some(editor) = ws.get_active() {
                        editor.replace_token(completion);
                    }
                }
                IdiomEvent::Snippet(snippet, cursor_offset) => {
                    if let Some(editor) = ws.get_active() {
                        editor.insert_snippet(snippet, cursor_offset);
                    };
                }
                IdiomEvent::WorkspaceEdit(edits) => ws.apply_edits(edits, self),
                IdiomEvent::Resize => {
                    ws.resize_all(self.editor_area.width, self.editor_area.height as usize);
                }
                IdiomEvent::Rebase => {
                    if let Some(editor) = ws.get_active() {
                        editor.rebase(self);
                    }
                    self.clear_popup();
                }
                IdiomEvent::Save => {
                    if let Some(editor) = ws.get_active() {
                        editor.save(self);
                    }
                    self.clear_popup();
                }
                IdiomEvent::CheckLSP(ft) => {
                    ws.check_lsp(ft, self).await;
                }
                IdiomEvent::SaveAndExit => {
                    ws.save_all(self);
                    self.exit = true;
                }
                IdiomEvent::Exit => {
                    self.exit = true;
                }
                IdiomEvent::FileUpdated(path) => {
                    ws.notify_update(path, self);
                }
                IdiomEvent::InsertText(insert) => {
                    if let Some(editor) = ws.get_active() {
                        editor.insert_text_with_relative_offset(insert);
                    };
                }
                IdiomEvent::FindSelector(pattern) => {
                    if let Some(editor) = ws.get_active() {
                        self.insert_mode();
                        self.popup(selector_ranges(editor.find_with_line(&pattern)));
                    } else {
                        self.clear_popup();
                    }
                }
                IdiomEvent::ActivateEditor(idx) => {
                    ws.activate_editor(idx, self);
                    self.clear_popup();
                    self.insert_mode();
                }
                IdiomEvent::FindToReplace(pattern, options) => {
                    self.popup(ReplacePopup::from_search(pattern, options));
                }
                IdiomEvent::GoToLine(idx) => {
                    if let Some(editor) = ws.get_active() {
                        editor.go_to(idx);
                    }
                    self.clear_popup();
                }
                IdiomEvent::GoToSelect { select: (from, to), clear_popup } => {
                    if let Some(editor) = ws.get_active() {
                        editor.go_to_select(from, to);
                        if clear_popup {
                            self.clear_popup();
                        } else {
                            editor.render(self);
                        }
                    } else {
                        self.clear_popup();
                    }
                }
                IdiomEvent::ReplaceAll(clip, ranges) => {
                    if let Some(editor) = ws.get_active() {
                        editor.mass_replace(ranges, clip);
                    }
                    self.clear_popup();
                }
                IdiomEvent::ReplaceNextSelect { new_text, select: (from, to), next_select } => {
                    if let Some(editor) = ws.get_active() {
                        editor.replace_select(from, to, new_text.as_str());
                        if let Some((from, to)) = next_select {
                            editor.go_to_select(from, to);
                            editor.render(self);
                        }
                    }
                }
            }
        }
        self.exit
    }
}
