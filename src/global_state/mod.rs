mod clipboard;
mod file_tree_events;
mod footer_events;
mod workspace_events;

use std::{collections::LinkedList, path::PathBuf};

pub use self::file_tree_events::TreeEvent;
pub use self::footer_events::FooterEvent;
pub use self::workspace_events::WorkspaceEvent;
use crate::footer::Footer;
use crate::popups::{
    popup_replace::ReplacePopup,
    popups_editor::selector_ranges,
    popups_tree::{search_tree_files, tree_file_selector},
    PopupInterface,
};
use crate::tree::Tree;
use crate::workspace::Workspace;
pub use clipboard::Clipboard;

use crossterm::event::KeyEvent;
use ratatui::{
    style::{Color, Style},
    text::Span,
    Frame,
};

#[derive(Default, Clone)]
pub enum PopupMessage {
    #[default]
    None,
    Tree(TreeEvent),
    Workspace(WorkspaceEvent),
    Clear,
}

#[derive(Default)]
pub enum Mode {
    #[default]
    Select,
    Insert,
}

#[derive(Default)]
pub struct GlobalState {
    pub mode: Mode,
    pub popup: Option<Box<dyn PopupInterface>>,
    pub footer: Vec<FooterEvent>,
    pub workspace: LinkedList<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
    pub clipboard: Clipboard,
    pub exit: bool,
}

impl GlobalState {
    pub async fn exchange_should_exit(
        &mut self,
        tree: &mut Tree,
        workspace: &mut Workspace,
        footer: &mut Footer,
    ) -> bool {
        self.exchange_tree(tree).await;
        self.exchange_ws(workspace).await;
        self.exchange_footer(footer);
        self.exit
    }

    pub fn mode_span(&self) -> Span<'static> {
        match self.mode {
            Mode::Insert => {
                let color = if self.popup.is_some() { Color::Gray } else { Color::Rgb(255, 0, 0) };
                Span::styled("  INSERT  ", Style::default().fg(color))
            }
            Mode::Select => {
                let color = if self.popup.is_some() { Color::Gray } else { Color::LightCyan };
                Span::styled("  SELECT  ", Style::default().fg(color))
            }
        }
    }

    // POPUP HANDLERS
    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        self.popup.replace(popup);
    }

    pub fn render_popup_if_exists(&mut self, frame: &mut Frame<'_>) {
        if let Some(popup) = self.popup.as_mut() {
            popup.render(frame)
        }
    }

    pub fn map_modal_if_exists(&mut self, key: &KeyEvent) -> bool {
        if let Some(popup) = self.popup.as_mut() {
            match popup.map(key, &mut self.clipboard) {
                PopupMessage::Clear => {
                    self.popup = None;
                }
                PopupMessage::None => {}
                PopupMessage::Tree(event) => {
                    self.tree.push(event);
                }
                PopupMessage::Workspace(event) => {
                    self.workspace.push_back(event);
                }
            }
            return true;
        }
        false
    }

    pub fn try_ws_event(&mut self, value: impl TryInto<WorkspaceEvent>) {
        if let Ok(event) = value.try_into() {
            self.workspace.push_back(event);
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

    pub fn exchange_footer(&mut self, footer: &mut Footer) {
        for event in self.footer.drain(..) {
            event.map(footer);
        }
    }

    // pub fn logged_ok<T>(&mut self, result: Result<T>) -> Option<T> {
    //     match result {
    //         Ok(val) => Some(val),
    //         Err(err) => {
    //             self.error(err.to_string());
    //             None
    //         }
    //     }
    // }

    /// Attempts to create new editor if err logs it and returns false else true.
    pub async fn try_new_editor(&mut self, workspace: &mut Workspace, path: PathBuf) -> bool {
        if let Err(err) = workspace.new_from(path, self).await {
            self.error(err.to_string());
            return false;
        }
        true
    }

    pub async fn exchange_tree(&mut self, tree: &mut Tree) {
        for event in self.tree.drain(..) {
            match event {
                TreeEvent::PopupAccess => {
                    if let Some(popup) = self.popup.as_mut() {
                        popup.update_tree(tree);
                    }
                }
                TreeEvent::SearchFiles(pattern) => {
                    self.popup = Some(search_tree_files(pattern));
                }
                TreeEvent::Open(path) => {
                    tree.select_by_path(&path);
                    self.popup = None;
                    self.workspace.push_back(WorkspaceEvent::Open(path, 0));
                }
                TreeEvent::OpenAtLine(path, line) => {
                    tree.select_by_path(&path);
                    self.popup = None;
                    self.workspace.push_back(WorkspaceEvent::Open(path, line));
                }
                TreeEvent::OpenAtSelect(path, select) => {
                    tree.select_by_path(&path);
                    self.workspace.push_back(WorkspaceEvent::Open(path, 0));
                    self.workspace.push_back(WorkspaceEvent::GoToSelect { select, should_clear: true });
                }
                TreeEvent::CreateFileOrFolder(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder(name) {
                        if !new_path.is_dir() {
                            self.workspace.push_back(WorkspaceEvent::Open(new_path, 0));
                            self.mode = Mode::Insert;
                        }
                    }
                    self.popup = None;
                }
                TreeEvent::CreateFileOrFolderBase(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder_base(name) {
                        if !new_path.is_dir() {
                            self.workspace.push_back(WorkspaceEvent::Open(new_path, 0));
                            self.mode = Mode::Insert;
                        }
                    }
                    self.popup = None;
                }
                TreeEvent::RenameFile(name) => {
                    if let Err(error) = tree.rename_file(name) {
                        self.footer.push(FooterEvent::Error(error.to_string()));
                    };
                    self.popup = None;
                }
                TreeEvent::SelectTreeFiles(pattern) => {
                    self.mode = Mode::Select;
                    self.popup = Some(tree_file_selector(tree.search_select_files(pattern).await));
                }
                TreeEvent::SelectTreeFilesFull(pattern) => {
                    self.mode = Mode::Select;
                    self.popup = Some(tree_file_selector(tree.search_files(pattern).await))
                }
            }
        }
    }

    pub async fn exchange_ws(&mut self, workspace: &mut Workspace) {
        while let Some(event) = self.workspace.pop_front() {
            match event {
                WorkspaceEvent::GoToLine(idx) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.go_to(idx);
                    }
                    self.popup = None;
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
                    self.popup = None;
                }
                WorkspaceEvent::GoToSelect { select: (from, to), should_clear } => {
                    if let Some(editor) = workspace.get_active() {
                        editor.go_to_select(from, to);
                        if should_clear {
                            self.popup = None;
                        }
                    } else {
                        self.popup = None;
                    }
                }
                WorkspaceEvent::ActivateEditor(idx) => {
                    workspace.state.select(Some(idx));
                    self.popup = None;
                }
                WorkspaceEvent::FindSelector(pattern) => {
                    if let Some(editor) = workspace.get_active() {
                        self.mode = Mode::Insert;
                        self.popup = Some(selector_ranges(editor.find_with_line(&pattern)));
                    } else {
                        self.popup = None;
                    }
                }
                WorkspaceEvent::FindToReplace(pattern, options) => {
                    self.popup.replace(ReplacePopup::from_search(pattern, options));
                }
                WorkspaceEvent::AutoComplete(completion) => {
                    if let Some(editor) = workspace.get_active() {
                        editor.replace_token(completion);
                    }
                }
                WorkspaceEvent::WorkspaceEdit(edits) => workspace.apply_edits(edits, self),
                WorkspaceEvent::Open(path, line) => {
                    if !path.is_dir() && workspace.new_at_line(path, line, self).await.is_ok() {
                        self.mode = Mode::Insert;
                    } else {
                        self.mode = Mode::Select;
                    }
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
    }
}