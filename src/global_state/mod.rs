use std::error::Error;
mod clipboard;
mod controls;
mod draw;
mod events;
mod message;

use crate::{
    configs::UITheme,
    popups::{
        popup_replace::ReplacePopup, popup_tree_search::ActiveFileSearch, popups_editor::selector_ranges,
        PopupInterface,
    },
    render::{
        backend::{color, Backend},
        layout::{Rect, DOUBLE_BORDERS},
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

const INSERT_SPAN: &'static str = "  --INSERT--   ";
const SELECT_SPAN: &'static str = "  --SELECT--   ";

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

type KeyMapCallback =
    fn(&mut GlobalState, &KeyEvent, &mut Workspace, &mut Tree, &mut EditorTerminal) -> std::io::Result<bool>;
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
    pub popup: Option<Box<dyn PopupInterface>>,
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
    pub clipboard: Clipboard,
    pub exit: bool,
    pub screen_rect: Rect,
    pub tree_area: Rect,
    pub tab_area: Rect,
    pub editor_area: Rect,
    pub footer_area: Rect,
    message: Messages,
    components: Components,
}

impl GlobalState {
    pub fn new(backend: Backend) -> std::io::Result<Self> {
        let mut new = Self {
            mode: Mode::default(),
            tree_size: 15,
            key_mapper: controls::map_tree,
            mouse_mapper: controls::mouse_handler,
            draw_callback: draw::draw_with_tree,
            theme: UITheme::new().unwrap_or_default(),
            writer: backend,
            popup: None,
            workspace: Vec::default(),
            tree: Vec::default(),
            clipboard: Clipboard::default(),
            exit: false,
            screen_rect: Backend::screen()?,
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_area: Rect::default(),
            message: Messages::new(),
            components: Components::default(),
        };
        new.recalc_draw_size();
        new.select_mode();
        Ok(new)
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

    pub fn render_stats(&mut self, len: usize, select_len: usize, cursor: CursorPosition) -> std::io::Result<()> {
        if let Some(mut line) = self.footer_area.get_line(0) {
            line += INSERT_SPAN.len();
            self.writer.set_style(self.theme.accent_style)?;
            let mut rev_builder = line.unsafe_builder_rev(&mut self.writer)?;
            if select_len != 0 {
                rev_builder.push(&format!(" ({select_len} selected)"))?;
            }
            rev_builder.push(&format!("  Doc Len {len}, Ln {}, Col {}", cursor.line + 1, cursor.char + 1))?;
            self.message.line = rev_builder.to_line();
            self.message.render(self.theme.accent_style, &mut self.writer)?;
            self.writer.reset_style()?;
        }
        Ok(())
    }

    fn find_draw_callback(&self) -> DrawCallback {
        let with_term = self.components.contains(Components::TERM);
        let with_popup = self.components.contains(Components::POPUP);
        if matches!(self.mode, Mode::Select) || self.components.contains(Components::TREE) {
            if with_term && with_popup {
                return draw::draw_full;
            }
            if self.components.contains(Components::TERM) {
                return draw::draw_with_tree_and_term;
            }
            if self.components.contains(Components::POPUP) {
                return draw::draw_with_tree_and_popup;
            }
            return draw::draw_with_tree;
        }
        if with_popup && with_term {
            return draw::draw_with_term_and_popup;
        }
        if with_term {
            return draw::draw_with_term;
        }
        if with_popup {
            return draw::draw_with_popup;
        }
        draw::draw
    }

    pub fn map_key(
        &mut self,
        event: &KeyEvent,
        workspace: &mut Workspace,
        tree: &mut Tree,
        tmux: &mut EditorTerminal,
    ) -> std::io::Result<bool> {
        (self.key_mapper)(self, event, workspace, tree, tmux)
    }

    pub fn map_mouse(&mut self, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
        (self.mouse_mapper)(self, event, tree, workspace)
    }

    pub fn select_mode(&mut self) {
        self.mode = Mode::Select;
        self.key_mapper = controls::map_tree;
        if !self.components.contains(Components::TREE) {
            self.recalc_draw_size();
        };
        if let Some(line) = self.footer_area.get_line(0) {
            self.writer.save_cursor().unwrap();
            let mut style = self.theme.accent_style;
            style.set_fg(Some(color::cyan()));
            style.add_bold();
            line.render_styled(SELECT_SPAN, style, &mut self.writer).unwrap();
            self.writer.restore_cursor().unwrap();
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.key_mapper = controls::map_editor;
        if !self.components.contains(Components::TREE) {
            self.recalc_draw_size();
        };
        if let Some(line) = self.footer_area.get_line(0) {
            self.writer.save_cursor().unwrap();
            let mut style = self.theme.accent_style;
            style.set_fg(Some(color::rgb(255, 0, 0)));
            style.add_bold();
            line.render_styled(INSERT_SPAN, style, &mut self.writer).unwrap();
            self.writer.restore_cursor().unwrap();
        };
    }

    pub fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
    }

    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        self.components.insert(Components::POPUP);
        self.key_mapper = controls::map_popup;
        self.mouse_mapper = controls::disable_mouse;
        self.popup.replace(popup);
    }

    pub fn clear_popup(&mut self) -> Option<Box<dyn PopupInterface>> {
        match self.mode {
            Mode::Select => {
                self.key_mapper = controls::map_tree;
            }
            Mode::Insert => {
                self.key_mapper = controls::map_editor;
            }
        }
        self.components.remove(Components::POPUP);
        self.mouse_mapper = controls::mouse_handler;
        self.popup.take()
    }

    pub fn toggle_tree(&mut self) {
        if self.components.contains(Components::TREE) {
            self.components.remove(Components::TREE);
            self.recalc_draw_size();
        } else {
            self.components.insert(Components::TREE);
            self.recalc_draw_size();
        }
    }

    pub fn expand_tree_size(&mut self) {
        self.tree_size = std::cmp::min(75, self.tree_size + 1);
        self.recalc_draw_size();
    }

    pub fn shrink_tree_size(&mut self) {
        self.tree_size = std::cmp::max(15, self.tree_size - 1);
        self.recalc_draw_size();
    }

    pub fn toggle_terminal(&mut self, runner: &mut EditorTerminal) {
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

    pub fn render_popup_if_exists(&mut self) -> std::io::Result<()> {
        let mut popup = self.popup.take();
        if let Some(popup) = popup.as_mut() {
            popup.render(self)?;
        };
        self.popup = popup;
        Ok(())
    }

    pub fn map_popup_if_exists(&mut self, key: &KeyEvent) -> bool {
        if let Some(popup) = self.popup.as_mut() {
            match popup.map(key, &mut self.clipboard) {
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
            return true;
        }
        false
    }

    pub fn try_tree_event(&mut self, value: impl TryInto<TreeEvent>) {
        if let Ok(event) = value.try_into() {
            self.tree.push(event);
        }
    }

    #[inline]
    pub fn message(&mut self, msg: impl Into<String>) {
        self.message.message(msg.into());
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.message.error(msg.into());
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.message.success(msg.into());
    }

    pub fn full_resize(&mut self, height: u16, width: u16, workspace: &mut Workspace) {
        self.screen_rect = (width, height).into();
        self.recalc_draw_size();
        workspace.resize_render(self.editor_area.width as usize, self.editor_area.height as usize);
    }

    pub fn recalc_draw_size(&mut self) {
        self.tree_area = self.screen_rect.clone();
        self.footer_area = self.tree_area.splitoff_rows(1);
        if let Some(mut line) = self.footer_area.get_line(0) {
            line += SELECT_SPAN.len();
            self.message.line = line;
        };
        if matches!(self.mode, Mode::Select) || self.components.contains(Components::TREE) {
            self.tab_area = self.tree_area.keep_col((self.tree_size * self.screen_rect.width) / 100);
            let _ = self.tree_area.top_border().right_border().draw_borders(
                Some(DOUBLE_BORDERS),
                Some(color::dark_grey()),
                &mut self.writer,
            );
        };
        self.editor_area = self.tab_area.keep_rows(1);
    }

    /// unwrap or default with logged error
    pub fn unwrap_default_result<T: Default, E: Error>(&mut self, result: Result<T, E>, prefix: &str) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                let mut msg = prefix.to_owned();
                msg.push_str(&error.to_string());
                self.error(msg);
                T::default()
            }
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
                    if let Some(popup) = self.popup.as_mut() {
                        popup.update_tree(tree);
                    }
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
                    if let Err(error) = tree.rename_file(name) {
                        self.message.error(error.to_string())
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
                    if let Some(popup) = self.popup.as_mut() {
                        popup.update_workspace(workspace);
                    }
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
                    workspace.resize_render(self.editor_area.width as usize, self.editor_area.height as usize);
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
        if matches!(self.mode, Mode::Select) {
            self.message.render(self.theme.accent_style, &mut self.writer).unwrap();
        }
        self.exit
    }
}
