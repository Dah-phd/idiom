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
    workspace::Workspace,
};
pub use clipboard::Clipboard;
use controls::map_term;
use crossterm::event::{KeyEvent, MouseEvent};
pub use events::{FooterEvent, TreeEvent, WorkspaceEvent};
use std::path::PathBuf;

use self::draw::Components;

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

pub struct GlobalState {
    mode: Mode,
    tree_size: usize,
    key_mapper: KeyMapCallback,
    mouse_mapper: MouseMapCallback,
    pub theme: UITheme,
    pub writer: Backend,
    pub popup: Option<Box<dyn PopupInterface>>,
    pub footer: Vec<FooterEvent>,
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
    pub clipboard: Clipboard,
    pub exit: bool,
    pub screen_rect: Rect,
    pub tree_area: Rect,
    pub tab_area: Rect,
    pub editor_area: Rect,
    pub footer_area: Rect,
    components: Components,
}

impl GlobalState {
    pub fn new(backend: Backend) -> std::io::Result<Self> {
        let screen_rect = crossterm::terminal::size()?.into();
        let mut new = Self {
            mode: Mode::default(),
            tree_size: 15,
            key_mapper: controls::map_tree,
            mouse_mapper: controls::mouse_handler,
            theme: UITheme::new().unwrap_or_default(),
            writer: backend,
            popup: None,
            footer: Vec::default(),
            workspace: Vec::default(),
            tree: Vec::default(),
            clipboard: Clipboard::default(),
            exit: false,
            screen_rect,
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_area: Rect::default(),
            components: Components::default(),
        };
        new.recalc_draw_size();
        new.select_mode();
        Ok(new)
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
        if let Some(mut line) = self.footer_area.get_line(0) {
            let _ = self.writer.save_cursor();
            let mut style = self.theme.accent_style;
            line.width = SELECT_SPAN.len();
            style.set_fg(Some(color::cyan()));
            style.add_bold();
            let _ = line.render_styled(SELECT_SPAN, style, &mut self.writer);
            let _ = self.writer.restore_cursor();
        };
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.key_mapper = controls::map_editor;
        if !self.components.contains(Components::TREE) {
            self.recalc_draw_size();
        };
        if let Some(mut line) = self.footer_area.get_line(0) {
            let _ = self.writer.save_cursor();
            let mut style = self.theme.accent_style;
            line.width = INSERT_SPAN.len();
            style.set_fg(Some(color::rgb(255, 0, 0)));
            style.add_bold();
            let _ = line.render_styled(INSERT_SPAN, style, &mut self.writer);
            let _ = self.writer.restore_cursor();
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
                    // self.mode_span.style = SELECT_STYLE;
                }
                Mode::Insert => {
                    self.key_mapper = controls::map_editor;
                    // self.mode_span.style = INSERT_STYLE;
                }
            }
            self.mouse_mapper = controls::mouse_handler;
        } else {
            self.components.insert(Components::TERM);
            runner.activate();
            self.key_mapper = map_term;
            self.mouse_mapper = controls::disable_mouse;
            // self.mode_span.style = MUTED_STYLE;
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

    pub fn message(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Message(msg.into()));
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Error(msg.into()));
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Success(msg.into()));
    }

    pub fn full_resize(&mut self, height: u16, width: u16, workspace: &mut Workspace) {
        self.screen_rect = (width, height).into();
        self.recalc_draw_size();
        workspace.resize_render(self.editor_area.width as usize, self.editor_area.height as usize);
    }

    pub fn recalc_draw_size(&mut self) {
        self.tree_area = self.screen_rect.clone();
        self.footer_area = self.tree_area.splitoff_rows(1);
        if matches!(self.mode, Mode::Select) || self.components.contains(Components::TREE) {
            self.tab_area = self.tree_area.keep_col((self.tree_size * self.screen_rect.width) / 100);
            let _ = self.tree_area.top_border().right_border().draw_borders(
                Some(DOUBLE_BORDERS),
                color::dark_grey(),
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
                        // footer.error(error.to_string());
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
        for event in self.footer.drain(..) {
            // event.map(footer);
        }
        self.exit
    }
}
