mod clipboard;
mod controls;
mod draw;
mod events;

use crate::{
    footer::Footer,
    popups::{
        popup_replace::ReplacePopup, popup_tree_search::ActiveFileSearch, popups_editor::selector_ranges,
        PopupInterface,
    },
    runner::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
};
pub use clipboard::Clipboard;
use controls::map_term;
use crossterm::event::{KeyEvent, MouseEvent};
pub use events::{FooterEvent, TreeEvent, WorkspaceEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    Frame,
};
use std::{borrow::Cow, path::PathBuf};

use self::draw::Components;

const INSERT_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Rgb(255, 0, 0));
const INSERT_SPAN: Span<'static> = Span { content: Cow::Borrowed(" --INSERT-- "), style: INSERT_STYLE };
const SELECT_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::LightCyan);
const SELECT_SPAN: Span<'static> = Span { content: Cow::Borrowed(" --SELECT-- "), style: SELECT_STYLE };
const MUTED_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::DarkGray);

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

pub struct GlobalState {
    mode: Mode,
    tree_size: u16,
    key_mapper: fn(&mut GlobalState, &KeyEvent, &mut Workspace, &mut Tree, &mut EditorTerminal) -> bool,
    mouse_mapper: fn(&mut Self, MouseEvent, &mut Tree, &mut Workspace),
    draw: fn(&mut Self, &mut Frame, &mut Workspace, &mut Tree, &mut Footer, &mut EditorTerminal),
    pub mode_span: Span<'static>,
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
    pub fn new(height: u16, width: u16) -> Self {
        Self {
            mode: Mode::default(),
            tree_size: 15,
            draw: draw::inactive_tmux,
            key_mapper: controls::map_tree,
            mouse_mapper: controls::mouse_handler,
            mode_span: SELECT_SPAN,
            popup: None,
            footer: Vec::default(),
            workspace: Vec::default(),
            tree: Vec::default(),
            clipboard: Clipboard::default(),
            exit: false,
            screen_rect: Rect { height, width, ..Default::default() },
            tree_area: Rect::default(),
            tab_area: Rect::default(),
            editor_area: Rect::default(),
            footer_area: Rect::default(),
            components: Components::EDITOR | Components::TREE,
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        workspace: &mut Workspace,
        tree: &mut Tree,
        footer: &mut Footer,
        tmux: &mut EditorTerminal,
    ) {
        (self.draw)(self, frame, workspace, tree, footer, tmux);
    }

    pub fn map_key(
        &mut self,
        event: &KeyEvent,
        workspace: &mut Workspace,
        tree: &mut Tree,
        tmux: &mut EditorTerminal,
    ) -> bool {
        (self.key_mapper)(self, event, workspace, tree, tmux)
    }

    pub fn map_mouse(&mut self, event: MouseEvent, tree: &mut Tree, workspace: &mut Workspace) {
        (self.mouse_mapper)(self, event, tree, workspace)
    }

    pub fn select_mode(&mut self) {
        self.mode = Mode::Select;
        self.key_mapper = controls::map_tree;
        self.mode_span = SELECT_SPAN;
    }

    pub fn insert_mode(&mut self) {
        self.mode = Mode::Insert;
        self.key_mapper = controls::map_editor;
        self.mode_span = INSERT_SPAN;
    }

    pub fn is_insert(&self) -> bool {
        matches!(self.mode, Mode::Insert)
    }

    pub fn popup(&mut self, popup: Box<dyn PopupInterface>) {
        self.key_mapper = controls::map_popup;
        self.mouse_mapper = controls::disable_mouse;
        self.popup.replace(popup);
        self.mode_span.style = MUTED_STYLE;
    }

    pub fn clear_popup(&mut self) -> Option<Box<dyn PopupInterface>> {
        match self.mode {
            Mode::Select => {
                self.key_mapper = controls::map_tree;
                self.mode_span.style = SELECT_STYLE;
            }
            Mode::Insert => {
                self.key_mapper = controls::map_editor;
                self.mode_span.style = INSERT_STYLE;
            }
        }
        self.mouse_mapper = controls::mouse_handler;
        self.popup.take()
    }

    pub fn toggle_tree(&mut self) {
        if self.components.contains(Components::TREE) {
            self.components.remove(Components::TREE);
            self.draw = draw::inactive_tree_and_tmux;
            let footer_layout = draw::layour_workspace_footer(self.screen_rect);
            self.footer_area = footer_layout[1];
            let workspace_layout = draw::layot_tabs_editor(footer_layout[0]);
            self.editor_area = workspace_layout[1];
            self.tab_area = workspace_layout[0];
            self.workspace.push(WorkspaceEvent::Resize);
        } else {
            self.components.insert(Components::TREE);
            self.draw = draw::inactive_tmux;
            self.recalc_editor_size();
        }
    }

    pub fn expand_tree_size(&mut self) {
        self.tree_size = std::cmp::min(75, self.tree_size + 1);
        self.recalc_editor_size();
    }

    pub fn shrink_tree_size(&mut self) {
        self.tree_size = std::cmp::max(15, self.tree_size - 1);
        self.recalc_editor_size();
    }

    pub fn toggle_terminal(&mut self, runner: &mut EditorTerminal) {
        if self.components.contains(Components::TERM) {
            self.components.remove(Components::TERM);
            self.draw = draw::inactive_tmux;
            match self.mode {
                Mode::Select => {
                    self.key_mapper = controls::map_tree;
                    self.mode_span.style = SELECT_STYLE;
                }
                Mode::Insert => {
                    self.key_mapper = controls::map_editor;
                    self.mode_span.style = INSERT_STYLE;
                }
            }
            self.mouse_mapper = controls::mouse_handler;
        } else {
            self.components.insert(Components::TERM);
            self.draw = draw::full_draw;
            runner.activate();
            self.key_mapper = map_term;
            self.mouse_mapper = controls::disable_mouse;
            self.mode_span.style = MUTED_STYLE;
        }
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

    pub fn recalc_editor_size(&mut self) {
        let tree_layout = draw::layout_tree(self.screen_rect, self.tree_size);
        let footer_layout = draw::layour_workspace_footer(tree_layout[1]);
        self.tree_area = tree_layout[0];
        self.footer_area = footer_layout[1];
        let workspace_layout = draw::layot_tabs_editor(footer_layout[0]);
        self.editor_area = workspace_layout[1];
        self.tab_area = workspace_layout[0];
        self.workspace.push(WorkspaceEvent::Resize);
    }

    pub fn full_resize(&mut self, height: u16, width: u16, workspace: &mut Workspace) {
        self.screen_rect = Rect { height, width, ..Default::default() };
        self.recalc_editor_size();
        workspace.resize_render(self.editor_area.width as usize, self.editor_area.bottom() as usize);
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
                    self.workspace.push(WorkspaceEvent::GoToSelect { select, should_clear: true });
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
                        footer.error(error.to_string());
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
                WorkspaceEvent::GoToSelect { select: (from, to), should_clear } => {
                    if let Some(editor) = workspace.get_active() {
                        editor.go_to_select(from, to);
                        if should_clear {
                            self.clear_popup();
                        }
                    } else {
                        self.clear_popup();
                    }
                }
                WorkspaceEvent::ActivateEditor(idx) => {
                    workspace.activate_editor(idx, Some(self));
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
                    }
                }
                WorkspaceEvent::Resize => {
                    workspace.resize_render(self.editor_area.width as usize, self.editor_area.bottom() as usize);
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
