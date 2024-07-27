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
pub use events::{TreeEvent, WorkspaceEvent};
use std::path::PathBuf;

use self::{draw::Components, message::Messages};

const INSERT_SPAN: &str = "  --INSERT--   ";
const SELECT_SPAN: &str = "  --SELECT--   ";

#[derive(Default, Clone)]
pub enum PopupMessage {
    #[default]
    None,
    Tree(TreeEvent),
    Workspace(WorkspaceEvent),
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
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
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
            workspace: Vec::default(),
            tree: Vec::default(),
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
        self.key_mapper = controls::map_tree;
        if !self.components.contains(Components::TREE) {
            self.draw_callback = draw::full_rebuild;
        };
        if let Some(line) = self.footer_area.get_line(0) {
            Mode::render_select_mode(line, self.theme.accent_style, &mut self.writer);
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.key_mapper = controls::map_editor;
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
        self.popup.render(gs);
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
                self.tree.push(event);
            }
            PopupMessage::Workspace(event) => {
                self.workspace.push(event);
            }
        }
        true
    }

    pub fn try_tree_event(&mut self, value: impl TryInto<TreeEvent>) {
        if let Ok(event) = value.try_into() {
            self.tree.push(event);
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
                self.workspace.push(WorkspaceEvent::CheckLSP(file_type));
            }
            _ => self.error(err.to_string()),
        }
    }

    /// Attempts to create new editor if err logs it and returns false else true.
    pub async fn try_new_editor(&mut self, workspace: &mut Workspace, path: PathBuf) -> bool {
        if let Err(err) = workspace.new_from(path, self).await {
            self.error(err.to_string());
            return false;
        }
        true
    }

    pub async fn exchange_should_exit(&mut self, tree: &mut Tree, workspace: &mut Workspace) -> bool {
        tree.finish_sync(self).await;
        for event in std::mem::take(&mut self.tree) {
            match event {
                TreeEvent::PopupAccess => {
                    self.popup.update_tree(tree);
                }
                TreeEvent::SearchFiles(pattern) => {
                    if pattern.len() > 1 {
                        let mut new_popup = ActiveFileSearch::new(pattern);
                        new_popup.update_tree(tree);
                        self.popup(new_popup);
                    } else {
                        self.popup(ActiveFileSearch::new(pattern));
                    }
                }
                TreeEvent::Open(path) => {
                    tree.select_by_path(&path);
                    self.clear_popup();
                    self.workspace.push(WorkspaceEvent::Open(path, 0));
                }
                TreeEvent::OpenAtLine(path, line) => {
                    tree.select_by_path(&path);
                    self.clear_popup();
                    self.workspace.push(WorkspaceEvent::Open(path, line));
                }
                TreeEvent::OpenAtSelect(path, select) => {
                    tree.select_by_path(&path);
                    self.workspace.push(WorkspaceEvent::Open(path, 0));
                    self.workspace.push(WorkspaceEvent::GoToSelect { select, clear_popup: true });
                }
                TreeEvent::SelectPath(path) => {
                    tree.select_by_path(&path);
                }
                TreeEvent::CreateFileOrFolder(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder(name) {
                        if !new_path.is_dir() {
                            self.workspace.push(WorkspaceEvent::Open(new_path, 0));
                            self.insert_mode();
                        }
                    }
                    self.clear_popup();
                }
                TreeEvent::CreateFileOrFolderBase(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder_base(name) {
                        if !new_path.is_dir() {
                            self.workspace.push(WorkspaceEvent::Open(new_path, 0));
                            self.insert_mode();
                        }
                    }
                    self.clear_popup();
                }
                TreeEvent::RenameFile(name) => {
                    if let Some(result) = tree.rename_path(name) {
                        match result {
                            Ok((old, new_path)) => workspace.rename_editors(old, new_path, self),
                            Err(err) => self.messages.error(err.to_string()),
                        }
                    };
                    self.clear_popup();
                }
                TreeEvent::RegisterLSP(lsp) => {
                    tree.lsp_register.push(lsp);
                }
            }
        }
        for event in std::mem::take(&mut self.workspace) {
            match event {
                WorkspaceEvent::GoToLine(idx) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.go_to(idx);
                    }
                    self.clear_popup();
                }
                WorkspaceEvent::PopupAccess => {
                    self.popup.update_workspace(workspace);
                }
                WorkspaceEvent::ReplaceNextSelect { new_text, select: (from, to), next_select } => {
                    if let Some(editor) = workspace.get_active() {
                        editor.replace_select(from, to, new_text.as_str());
                        if let Some((from, to)) = next_select {
                            editor.go_to_select(from, to);
                        }
                    }
                }
                WorkspaceEvent::ReplaceAll(clip, ranges) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.mass_replace(ranges, clip);
                    }
                    self.clear_popup();
                }
                WorkspaceEvent::GoToSelect { select: (from, to), clear_popup } => {
                    if let Some(editor) = workspace.get_active() {
                        editor.go_to_select(from, to);
                        if clear_popup {
                            self.clear_popup();
                        }
                    } else {
                        self.clear_popup();
                    }
                }
                WorkspaceEvent::ActivateEditor(idx) => {
                    workspace.activate_editor(idx, self);
                    self.clear_popup();
                    self.insert_mode();
                }
                WorkspaceEvent::FindSelector(pattern) => {
                    if let Some(editor) = workspace.get_active() {
                        self.insert_mode();
                        self.popup(selector_ranges(editor.find_with_line(&pattern)));
                    } else {
                        self.clear_popup();
                    }
                }
                WorkspaceEvent::FindToReplace(pattern, options) => {
                    self.popup(ReplacePopup::from_search(pattern, options));
                }
                WorkspaceEvent::AutoComplete(completion) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.replace_token(completion);
                    }
                }
                WorkspaceEvent::WorkspaceEdit(edits) => workspace.apply_edits(edits, self),
                WorkspaceEvent::Open(path, line) => {
                    if !path.is_dir() && workspace.new_at_line(path, line, self).await.is_ok() {
                        self.insert_mode();
                    } else {
                        self.select_mode();
                    };
                }
                WorkspaceEvent::InsertText(insert) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.insert_text_with_relative_offset(insert);
                    };
                }
                WorkspaceEvent::Snippet(snippet, cursor_offset) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.insert_snippet(snippet, cursor_offset);
                    };
                }
                WorkspaceEvent::Resize => {
                    workspace.resize_all(self.editor_area.width, self.editor_area.height as usize);
                }
                WorkspaceEvent::CheckLSP(ft) => {
                    workspace.check_lsp(ft, self).await;
                }
                WorkspaceEvent::SaveAndExit => {
                    workspace.save_all(self);
                    self.exit = true;
                }
                WorkspaceEvent::Exit => {
                    self.exit = true;
                }
            }
        }
        self.exit
    }
}
