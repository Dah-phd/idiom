pub mod actions;
pub mod cursor;
pub mod file;
pub mod utils;
use crate::{
    configs::{EditorAction, EditorConfigs, EditorKeyMap, FileType},
    global_state::{GlobalState, TreeEvent},
    lsp::LSP,
    utils::{REVERSED, UNDERLINED},
};
pub use cursor::CursorPosition;
pub use file::{DocStats, Editor};

use anyhow::Result;
use crossterm::event::KeyEvent;
use lsp_types::{DocumentChangeOperation, DocumentChanges, OneOf, ResourceOp, TextDocumentEdit, WorkspaceEdit};
use ratatui::layout::Direction;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Tabs,
    Frame,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    rc::Rc,
};

const TAB_HIGHTLIGHT: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED);
const RECT_CONSTRAINT: [Constraint; 2] = [Constraint::Length(1), Constraint::Percentage(100)];

pub struct Workspace {
    pub editors: Vec<Editor>,
    base_config: EditorConfigs,
    key_map: EditorKeyMap,
    lsp_servers: HashMap<FileType, LSP>,
    tab_style: Style,
    map_callback: fn(&mut Self, &KeyEvent, &mut GlobalState) -> bool,
}

impl Workspace {
    pub fn new(key_map: EditorKeyMap) -> Self {
        Self {
            editors: Vec::default(),
            base_config: EditorConfigs::new(),
            key_map,
            tab_style: UNDERLINED,
            lsp_servers: HashMap::new(),
            map_callback: map_editor,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, gs: &mut GlobalState) {
        if let Some(file) = self.editors.get_mut(0) {
            let editor_widget = file.collect_widget(gs);
            frame.render_widget(editor_widget, gs.editor_area);

            file.lexer.render_modal_if_exist(frame, gs.editor_area, &file.cursor);

            let titles: Vec<_> = self.editors.iter().map(|e| e.display.to_owned()).collect();

            let tabs = Tabs::new(titles).style(self.tab_style).highlight_style(TAB_HIGHTLIGHT).select(0);
            frame.render_widget(tabs, gs.tab_area);
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        (self.map_callback)(self, key, gs)
    }

    pub fn toggle_tabs(&mut self) {
        self.map_callback = map_tabs;
        self.tab_style = REVERSED;
    }

    pub fn toggle_editor(&mut self) {
        self.map_callback = map_editor;
        self.tab_style = UNDERLINED;
    }

    pub fn resize_render(&mut self, width: usize, height: usize) {
        for editor in self.editors.iter_mut() {
            editor.resize(width, height);
        }
    }

    pub fn get_stats(&self) -> Option<DocStats> {
        self.editors.first().map(|editor| editor.get_stats())
    }

    pub fn tabs(&self) -> Vec<String> {
        self.editors.iter().map(|editor| editor.display.to_owned()).collect()
    }

    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.first_mut()
    }

    pub fn activate_editor(&mut self, idx: usize, gs: Option<&mut GlobalState>) {
        if idx < self.editors.len() {
            let editor = self.editors.remove(idx);
            if let Some(state) = gs {
                state.tree.push(TreeEvent::SelectPath(editor.path.clone()));
            }
            self.editors.insert(0, editor);
        }
    }

    pub fn apply_edits(&mut self, edits: WorkspaceEdit, events: &mut GlobalState) {
        if let Some(edits) = edits.changes {
            for (file_url, file_edits) in edits {
                if let Some(editor) = self.get_editor(file_url.path()) {
                    editor.apply_file_edits(file_edits);
                } else if let Ok(mut editor) = self.build_basic_editor(PathBuf::from(file_url.path())) {
                    editor.apply_file_edits(file_edits);
                    editor.try_write_file(events);
                } else {
                    events.error(format!("Unable to build editor for {}", file_url.path()));
                }
            }
        }
        if let Some(documet_edit) = edits.document_changes {
            match documet_edit {
                DocumentChanges::Edits(edits) => {
                    for text_document_edit in edits {
                        self.handle_text_document_edit(text_document_edit, events);
                    }
                }
                DocumentChanges::Operations(operations) => {
                    for operation in operations {
                        match operation {
                            DocumentChangeOperation::Edit(text_document_edit) => {
                                self.handle_text_document_edit(text_document_edit, events);
                            }
                            DocumentChangeOperation::Op(operation) => {
                                if let Err(err) = self.handle_tree_operations(operation) {
                                    events.error(format!("Failed file tree operation: {err}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_text_document_edit(&mut self, mut text_document_edit: TextDocumentEdit, events: &mut GlobalState) {
        if let Some(editor) = self.get_editor(text_document_edit.text_document.uri.path()) {
            let edits = text_document_edit
                .edits
                .drain(..)
                .map(|edit| match edit {
                    OneOf::Left(edit) => edit,
                    OneOf::Right(annotated) => annotated.text_edit,
                })
                .collect();
            editor.apply_file_edits(edits);
        } else if let Ok(mut editor) =
            self.build_basic_editor(PathBuf::from(text_document_edit.text_document.uri.path()))
        {
            let edits = text_document_edit
                .edits
                .drain(..)
                .map(|edit| match edit {
                    OneOf::Left(edit) => edit,
                    OneOf::Right(annotated) => annotated.text_edit,
                })
                .collect();
            editor.apply_file_edits(edits);
            editor.try_write_file(events);
        } else {
            events.error(format!("Unable to build editor for {}", text_document_edit.text_document.uri.path()));
        };
    }

    fn handle_tree_operations(&mut self, operation: ResourceOp) -> Result<()> {
        match operation {
            ResourceOp::Create(create) => {
                let path = PathBuf::from(create.uri.path());
                if path.exists() {
                    if let Some(options) = create.options {
                        if matches!(options.overwrite, Some(overwrite) if !overwrite)
                            || matches!(options.ignore_if_exists, Some(ignore) if ignore)
                        {
                            return Err(anyhow::anyhow!("File {path:?} already exists!"));
                        }
                    }
                };
                std::fs::write(path, "")?;
            }
            ResourceOp::Delete(delete) => {
                let search_path = PathBuf::from(delete.uri.path()).canonicalize()?;
                if search_path.is_file() {
                    std::fs::remove_file(search_path)?;
                } else {
                    std::fs::remove_dir_all(search_path)?;
                }
            }
            ResourceOp::Rename(rename) => {
                std::fs::rename(rename.old_uri.path(), rename.new_uri.path())?;
                if let Some(editor) = self.get_editor(rename.old_uri.path()) {
                    let path = PathBuf::from(rename.new_uri.path());
                    editor.display = path.display().to_string();
                    editor.path = path;
                }
            }
        }
        Ok(())
    }

    fn get_editor<T: Into<PathBuf>>(&mut self, path: T) -> Option<&mut Editor> {
        let path: PathBuf = path.into();
        self.editors.iter_mut().find(|editor| editor.path == path)
    }

    fn build_basic_editor(&mut self, file_path: PathBuf) -> Result<Editor> {
        Ok(Editor::from_path(file_path, self.base_config.clone())?)
    }

    async fn build_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> Result<Editor> {
        let mut new = Editor::from_path(file_path, self.base_config.clone())?;
        match self.lsp_servers.entry(new.file_type) {
            Entry::Vacant(entry) => {
                if let Ok(lsp) = LSP::from(&new.file_type).await {
                    new.lexer.set_lsp_client(lsp.aquire_client(), &new.file_type, new.stringify(), gs);
                    for editor in self.editors.iter_mut().filter(|e| e.file_type == new.file_type) {
                        editor.lexer.set_lsp_client(lsp.aquire_client(), &editor.file_type, editor.stringify(), gs);
                    }
                    entry.insert(lsp);
                }
            }
            Entry::Occupied(entry) => {
                new.lexer.set_lsp_client(entry.get().aquire_client(), &new.file_type, new.stringify(), gs);
            }
        }
        new.resize(gs.editor_area.width as usize, gs.editor_area.bottom() as usize);
        Ok(new)
    }

    pub async fn new_from(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> Result<()> {
        let file_path = file_path.canonicalize()?;
        if let Some(idx) =
            self.editors.iter().enumerate().find(|(_, editor)| editor.path == file_path).map(|(idx, _)| idx)
        {
            let editor = self.editors.remove(idx);
            self.editors.insert(0, editor);
            return Ok(());
        }
        let editor = self.build_editor(file_path, gs).await?;
        self.editors.insert(0, editor);
        self.toggle_editor();
        Ok(())
    }

    pub async fn new_at_line(&mut self, file_path: PathBuf, line: usize, gs: &mut GlobalState) -> Result<()> {
        self.new_from(file_path, gs).await?;
        if let Some(editor) = self.get_active() {
            editor.go_to(line);
        }
        Ok(())
    }

    pub fn select_tab_mouse(&mut self, col_idx: usize) -> Option<usize> {
        self.toggle_tabs();
        let mut cols_len = 0;
        for (editor_idx, editor) in self.editors.iter().enumerate() {
            cols_len += editor.display.len() + 3;
            if col_idx < cols_len {
                return Some(editor_idx);
            };
        }
        None
    }

    pub async fn check_lsp(&mut self, ft: FileType, gs: &mut GlobalState) {
        if let Some(lsp) = self.lsp_servers.get_mut(&ft) {
            match lsp.check_status().await {
                Ok(data) => match data {
                    None => gs.success("LSP function is normal".to_owned()),
                    Some(err) => {
                        self.full_sync(&ft, gs).await;
                        gs.success(format!("LSP recoved after: {err}"));
                    }
                },
                Err(err) => gs.error(err.to_string()),
            }
        }
    }

    pub async fn full_sync(&mut self, ft: &FileType, gs: &mut GlobalState) {
        if let Some(lsp) = self.lsp_servers.get(ft) {
            for editor in self.editors.iter_mut().filter(|e| &e.file_type == ft) {
                editor.lexer.set_lsp_client(lsp.aquire_client(), ft, editor.stringify(), gs);
            }
        }
    }

    pub fn close_active(&mut self) {
        if self.editors.is_empty() {
            return;
        }
        let editor = self.editors.remove(0);
        if let Some(mut client) = editor.lexer.lsp_client {
            let _ = client.file_did_close(&editor.path);
        }
    }

    pub fn are_updates_saved(&self) -> bool {
        for editor in self.editors.iter() {
            if !editor.is_saved() {
                return false;
            }
        }
        true
    }

    pub fn save(&mut self, gs: &mut GlobalState) {
        if let Some(editor) = self.get_active() {
            editor.save(gs);
        }
    }

    pub fn save_all(&mut self, events: &mut GlobalState) {
        for editor in self.editors.iter_mut() {
            editor.save(events);
        }
    }

    pub async fn refresh_cfg(&mut self, new_key_map: EditorKeyMap, gs: &mut GlobalState) {
        self.key_map = new_key_map;
        self.base_config.refresh();
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_config);
            if let Some(lsp) = self.lsp_servers.get(&editor.file_type) {
                if editor.lexer.lsp_client.is_none() {
                    editor.lexer.set_lsp_client(lsp.aquire_client(), &editor.file_type, editor.stringify(), gs);
                }
            }
        }
    }

    pub async fn graceful_exit(&mut self) {
        for (_, lsp) in self.lsp_servers.iter_mut() {
            let _ = lsp.graceful_exit().await;
        }
    }
}

/// handels keybindings for editor
fn map_editor(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    let action = ws.key_map.map(key);
    if let Some(editor) = ws.get_active() {
        if editor.lexer.map_modal_if_exists(key, gs) {
            return true;
        };
        if let Some(action) = action {
            match action {
                EditorAction::Char(ch) => editor.push(ch),
                EditorAction::NewLine => editor.new_line(),
                EditorAction::Indent => editor.indent(),
                EditorAction::Backspace => editor.backspace(),
                EditorAction::Delete => editor.del(),
                EditorAction::IndentStart => editor.indent_start(),
                EditorAction::Unintent => editor.unindent(),
                EditorAction::Up => editor.up(),
                EditorAction::Down => editor.down(),
                EditorAction::Left => editor.left(),
                EditorAction::Right => editor.right(),
                EditorAction::SelectUp => editor.select_up(),
                EditorAction::SelectDown => editor.select_down(),
                EditorAction::SelectLeft => editor.select_left(),
                EditorAction::SelectRight => editor.select_right(),
                EditorAction::SelectToken => editor.select_token(),
                EditorAction::SelectAll => editor.select_all(),
                EditorAction::ScrollUp => editor.scroll_up(),
                EditorAction::ScrollDown => editor.scroll_down(),
                EditorAction::SwapUp => editor.swap_up(),
                EditorAction::SwapDown => editor.swap_down(),
                EditorAction::JumpLeft => editor.jump_left(),
                EditorAction::JumpLeftSelect => editor.jump_left_select(),
                EditorAction::JumpRight => editor.jump_right(),
                EditorAction::JumpRightSelect => editor.jump_right_select(),
                EditorAction::EndOfLine => editor.end_of_line(),
                EditorAction::EndOfFile => editor.end_of_file(),
                EditorAction::StartOfLine => editor.start_of_line(),
                EditorAction::StartOfFile => editor.start_of_file(),
                EditorAction::FindReferences => editor.references(),
                EditorAction::GoToDeclaration => editor.declarations(),
                EditorAction::Help => editor.help(),
                EditorAction::LSPRename => editor.start_renames(),
                EditorAction::Undo => editor.undo(),
                EditorAction::Redo => editor.redo(),
                EditorAction::Save => editor.save(gs),
                EditorAction::Cancel => {
                    if editor.cursor.select_take().is_some() {
                        return true;
                    }
                    if ws.editors.len() > 1 {
                        ws.toggle_tabs();
                        return true;
                    }
                    return false;
                }
                EditorAction::Paste => {
                    if let Some(clip) = gs.clipboard.pull() {
                        editor.paste(clip);
                    }
                }
                EditorAction::Cut => {
                    if let Some(clip) = editor.cut() {
                        gs.clipboard.push(clip);
                    }
                }
                EditorAction::Copy => {
                    if let Some(clip) = editor.copy() {
                        gs.clipboard.push(clip);
                    }
                }
                EditorAction::Close => {
                    ws.close_active();
                    match ws.get_active() {
                        Some(editor) => gs.tree.push(TreeEvent::SelectPath(editor.path.clone())),
                        None => gs.select_mode(),
                    }
                }
            }
            return true;
        }
    }
    false
}

/// Handles keybinding while on tabs
fn map_tabs(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    if let Some(action) = ws.key_map.map(key) {
        if ws.editors.is_empty() {
            gs.select_mode();
            return false;
        }
        match action {
            EditorAction::NewLine => {
                ws.toggle_editor();
            }
            EditorAction::Up | EditorAction::Down => {
                ws.toggle_editor();
                gs.select_mode();
                return false;
            }
            EditorAction::Right | EditorAction::Indent => {
                let editor = ws.editors.remove(0);
                ws.editors.push(editor);
                gs.tree.push(TreeEvent::SelectPath(ws.editors[0].path.clone()));
            }
            EditorAction::Left | EditorAction::Unintent => {
                if let Some(editor) = ws.editors.pop() {
                    gs.tree.push(TreeEvent::SelectPath(editor.path.clone()));
                    ws.editors.insert(0, editor);
                }
            }
            EditorAction::Cancel => {
                ws.toggle_editor();
                return false;
            }
            EditorAction::Close => {
                ws.close_active();
            }
            _ => (),
        }
        return true;
    }
    false
}

pub fn layot_tabs_editor(area: Rect) -> Rc<[Rect]> {
    Layout::new(Direction::Vertical, RECT_CONSTRAINT).split(area)
}

#[cfg(test)]
mod test;
