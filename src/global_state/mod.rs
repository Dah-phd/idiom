mod clipboard;
mod events;
mod mouse;

use crate::{
    footer::{footer_render_area, Footer},
    popups::popup_replace::ReplacePopup,
    popups::PopupInterface,
    popups::{popup_tree_search::ActiveFileSearch, popups_editor::selector_ranges},
    tree::Tree,
    workspace::Workspace,
};
pub use clipboard::Clipboard;
pub use events::{FooterEvent, TreeEvent, WorkspaceEvent};

use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    Frame,
};
use std::path::PathBuf;

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
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
    pub clipboard: Clipboard,
    pub exit: bool,
    pub screen_size: Rect,
    pub editor_height: usize,
    pub editor_width: usize,
}

impl GlobalState {
    pub fn mode_span(&self) -> Span<'static> {
        match self.mode {
            Mode::Insert => {
                let color = if self.popup.is_some() { Color::Gray } else { Color::Rgb(255, 0, 0) };
                Span::styled(
                    " --INSERT-- ",
                    Style { fg: Some(color), add_modifier: Modifier::BOLD, ..Default::default() },
                )
            }
            Mode::Select => {
                let color = if self.popup.is_some() { Color::Gray } else { Color::LightCyan };
                Span::styled(
                    " --SELECT-- ",
                    Style { fg: Some(color), add_modifier: Modifier::BOLD, ..Default::default() },
                )
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

    pub fn map_popup_if_exists(&mut self, key: &KeyEvent) -> bool {
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

    pub fn recalc_editor_size(&mut self, tree: &Tree) {
        let tree_layout = tree.render_layout(self.screen_size);
        let remaining_screen = tree_layout[1];
        let footer_layout = footer_render_area(remaining_screen);
        let editor_screen = footer_layout[0];
        self.editor_height = editor_screen.bottom() as usize;
        self.editor_width = editor_screen.width as usize;
        self.workspace.push(WorkspaceEvent::Resize)
    }

    /// Attempts to create new editor if err logs it and returns false else true.
    pub async fn try_new_editor(&mut self, workspace: &mut Workspace, path: PathBuf) -> bool {
        if let Err(err) = workspace.new_from(path, self).await {
            self.error(err.to_string());
            return false;
        }
        true
    }

    pub async fn exchange_should_exit(
        &mut self,
        tree: &mut Tree,
        workspace: &mut Workspace,
        footer: &mut Footer,
    ) -> bool {
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
                        self.popup = Some(new_popup);
                    } else {
                        self.popup.replace(ActiveFileSearch::new(pattern));
                    }
                }
                TreeEvent::Open(path) => {
                    tree.select_by_path(&path);
                    self.popup = None;
                    self.workspace.push(WorkspaceEvent::Open(path, 0));
                }
                TreeEvent::OpenAtLine(path, line) => {
                    tree.select_by_path(&path);
                    self.popup = None;
                    self.workspace.push(WorkspaceEvent::Open(path, line));
                }
                TreeEvent::OpenAtSelect(path, select) => {
                    tree.select_by_path(&path);
                    self.workspace.push(WorkspaceEvent::Open(path, 0));
                    self.workspace.push(WorkspaceEvent::GoToSelect { select, should_clear: true });
                }
                TreeEvent::SelectPath(path) => {
                    tree.select_by_path(&path);
                }
                TreeEvent::CreateFileOrFolder(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder(name) {
                        if !new_path.is_dir() {
                            self.workspace.push(WorkspaceEvent::Open(new_path, 0));
                            self.mode = Mode::Insert;
                        }
                    }
                    self.popup = None;
                }
                TreeEvent::CreateFileOrFolderBase(name) => {
                    if let Ok(new_path) = tree.create_file_or_folder_base(name) {
                        if !new_path.is_dir() {
                            self.workspace.push(WorkspaceEvent::Open(new_path, 0));
                            self.mode = Mode::Insert;
                        }
                    }
                    self.popup = None;
                }
                TreeEvent::RenameFile(name) => {
                    if let Err(error) = tree.rename_file(name) {
                        footer.error(error.to_string());
                    };
                    self.popup = None;
                }
                TreeEvent::Resize { height, width } => {
                    self.screen_size = Rect { height, width, ..Default::default() };
                    self.recalc_editor_size(tree);
                }
            }
        }
        for event in std::mem::take(&mut self.workspace) {
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
                    workspace.activate_editor(idx, Some(self));
                    self.popup = None;
                    self.mode = Mode::Insert;
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
                WorkspaceEvent::Resize => {
                    workspace.resize_render(self.editor_width, self.editor_height);
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
            event.map(footer);
        }
        self.exit
    }
}
