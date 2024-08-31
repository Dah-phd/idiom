pub mod actions;
pub mod cursor;
pub mod editor;
pub mod line;
pub mod utils;
use crate::{
    configs::{EditorAction, EditorConfigs, EditorKeyMap, FileType},
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, TreeEvent},
    lsp::LSP,
    render::backend::{color, BackendProtocol, Style},
    utils::TrackedList,
};
use crossterm::event::KeyEvent;
pub use cursor::CursorPosition;
pub use editor::CodeEditor;
use lsp_types::{DocumentChangeOperation, DocumentChanges, OneOf, ResourceOp, TextDocumentEdit, WorkspaceEdit};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
};

/// implement Drop to attempt keep state upon close/crash
pub struct Workspace {
    editors: TrackedList<CodeEditor>,
    base_config: EditorConfigs,
    key_map: EditorKeyMap,
    tab_style: Style,
    lsp_servers: HashMap<FileType, LSP>,
    map_callback: fn(&mut Self, &KeyEvent, &mut GlobalState) -> bool,
}

impl Workspace {
    pub async fn new(key_map: EditorKeyMap, base_tree_paths: Vec<String>, gs: &mut GlobalState) -> Self {
        let mut base_config = gs.unwrap_or_default(EditorConfigs::new(), ".config: ");
        let mut lsp_servers = HashMap::new();
        for (ft, lsp_cmd) in base_config.derive_lsp_preloads(base_tree_paths, gs) {
            gs.success(format!("Preloading {lsp_cmd}"));
            if let Ok(lsp) = LSP::new(lsp_cmd).await {
                gs.tree.push(TreeEvent::RegisterLSP(lsp.borrow_client().get_lsp_registration()));
                lsp_servers.insert(ft, lsp);
            };
        }
        let tab_style = Style::fg(color::dark_yellow());
        Self { editors: TrackedList::new(), base_config, key_map, lsp_servers, map_callback: map_editor, tab_style }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        if let Some(editor) = self.editors.get_mut(0) {
            let line = match gs.tab_area.into_iter().next() {
                Some(line) => line,
                None => return,
            };
            gs.writer.save_cursor();
            gs.writer.set_style(Style::underlined(None));
            {
                let mut builder = line.unsafe_builder(&mut gs.writer);
                builder.push_styled(&editor.display, self.tab_style);
                for editor in self.editors.iter().skip(1) {
                    if !builder.push(" | ") || !builder.push(&editor.display) {
                        break;
                    };
                }
            }
            gs.writer.reset_style();
            gs.writer.restore_cursor();
        } else if let Some(line) = gs.tab_area.into_iter().next() {
            line.render_empty(&mut gs.writer);
        }
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.editors.collect_status() {
            self.render(gs);
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        (self.map_callback)(self, key, gs)
    }

    pub fn toggle_tabs(&mut self) {
        self.editors.mark_updated();
        self.map_callback = map_tabs;
        self.tab_style = Style::reversed();
    }

    pub fn toggle_editor(&mut self) {
        self.editors.mark_updated();
        self.map_callback = map_editor;
        self.tab_style = Style::fg(color::dark_yellow());
    }

    #[inline]
    pub fn resize_all(&mut self, width: usize, height: usize) {
        for editor in self.editors.iter_mut() {
            editor.resize(width, height);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.editors.is_empty()
    }

    pub fn tabs(&self) -> Vec<String> {
        self.editors.iter().map(|editor| editor.display.to_owned()).collect()
    }

    #[inline(always)]
    pub fn get_active(&mut self) -> Option<&mut CodeEditor> {
        self.editors.get_mut_no_update(0)
    }

    #[inline]
    pub fn rename_editors(&mut self, old: PathBuf, new_path: PathBuf, gs: &mut GlobalState) {
        if new_path.is_dir() {
            for editor in self.editors.iter_mut() {
                if editor.path.starts_with(&old) {
                    let mut updated_path = PathBuf::new();
                    let mut old = editor.path.iter();
                    for (new_part, ..) in new_path.iter().zip(&mut old) {
                        updated_path.push(new_part);
                    }
                    for remaining_part in old {
                        updated_path.push(remaining_part)
                    }
                    gs.log_if_lsp_error(editor.update_path(updated_path), editor.file_type);
                }
            }
        } else if let Some(editor) = self.editors.find(|e| e.path == old) {
            gs.log_if_lsp_error(editor.update_path(new_path), editor.file_type);
        }
    }

    pub fn activate_editor(&mut self, idx: usize, gs: &mut GlobalState) {
        if idx < self.editors.len() {
            let mut editor = self.editors.remove(idx);
            editor.clear_screen_cache();
            gs.tree.push(TreeEvent::SelectPath(editor.path.clone()));
            self.editors.insert(0, editor);
        }
    }

    pub fn apply_edits(&mut self, edits: WorkspaceEdit, gs: &mut GlobalState) {
        if let Some(edits) = edits.changes {
            for (file_url, file_edits) in edits {
                if let Some(editor) = self.get_editor(file_url.path().as_str()) {
                    editor.apply_file_edits(file_edits);
                } else if let Ok(mut editor) = self.build_basic_editor(PathBuf::from(file_url.path().as_str()), gs) {
                    editor.apply_file_edits(file_edits);
                    editor.try_write_file(gs);
                } else {
                    gs.error(format!("Unable to build editor for {}", file_url.path()));
                }
            }
        }
        if let Some(documet_edit) = edits.document_changes {
            match documet_edit {
                DocumentChanges::Edits(edits) => {
                    for text_document_edit in edits {
                        self.handle_text_document_edit(text_document_edit, gs);
                    }
                }
                DocumentChanges::Operations(operations) => {
                    for operation in operations {
                        match operation {
                            DocumentChangeOperation::Edit(text_document_edit) => {
                                self.handle_text_document_edit(text_document_edit, gs);
                            }
                            DocumentChangeOperation::Op(operation) => {
                                if let Err(err) = self.handle_tree_operations(operation) {
                                    gs.error(format!("Failed file tree operation: {err}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_text_document_edit(&mut self, mut text_document_edit: TextDocumentEdit, gs: &mut GlobalState) {
        if let Some(editor) = self.get_editor(text_document_edit.text_document.uri.path().as_str()) {
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
            self.build_basic_editor(PathBuf::from(text_document_edit.text_document.uri.path().as_str()), gs)
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
            editor.try_write_file(gs);
        } else {
            gs.error(format!("Unable to build editor for {}", text_document_edit.text_document.uri.path()));
        };
    }

    fn handle_tree_operations(&mut self, operation: ResourceOp) -> IdiomResult<()> {
        match operation {
            ResourceOp::Create(create) => {
                let path = PathBuf::from(create.uri.path().as_str());
                if path.exists() {
                    if let Some(options) = create.options {
                        if matches!(options.overwrite, Some(overwrite) if !overwrite)
                            || matches!(options.ignore_if_exists, Some(ignore) if ignore)
                        {
                            return Err(IdiomError::io_err(format!("File {path:?} already exists!")));
                        }
                    }
                };
                std::fs::write(path, "")?;
            }
            ResourceOp::Delete(delete) => {
                let search_path = PathBuf::from(delete.uri.as_str()).canonicalize()?;
                if search_path.is_file() {
                    std::fs::remove_file(search_path)?;
                } else {
                    std::fs::remove_dir_all(search_path)?;
                }
            }
            ResourceOp::Rename(rename) => {
                std::fs::rename(rename.old_uri.path().as_str(), rename.new_uri.path().as_str())?;
                if let Some(editor) = self.get_editor(rename.old_uri.path().as_str()) {
                    let path = PathBuf::from(rename.new_uri.path().as_str());
                    editor.display = path.display().to_string();
                    editor.path = path;
                }
            }
        }
        Ok(())
    }

    fn get_editor<T: Into<PathBuf>>(&mut self, path: T) -> Option<&mut CodeEditor> {
        let path: PathBuf = path.into();
        self.editors.iter_mut().find(|editor| editor.path == path)
    }

    fn build_basic_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<CodeEditor> {
        CodeEditor::from_path(file_path, &self.base_config, gs)
    }

    async fn build_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<CodeEditor> {
        let mut new = CodeEditor::from_path(file_path, &self.base_config, gs)?;
        new.resize(gs.editor_area.width, gs.editor_area.height as usize);
        let lsp_cmd = match self.base_config.derive_lsp(&new.file_type) {
            None => return Ok(new),
            Some(cmd) => cmd,
        };
        match self.lsp_servers.entry(new.file_type) {
            Entry::Vacant(entry) => {
                if let Ok(lsp) = LSP::new(lsp_cmd).await {
                    let client = lsp.aquire_client();
                    gs.tree.push(TreeEvent::RegisterLSP(client.get_lsp_registration()));
                    new.lexer.set_lsp_client(client, new.stringify(), gs);
                    for editor in self.editors.iter_mut().filter(|e| e.file_type == new.file_type) {
                        editor.lexer.set_lsp_client(lsp.aquire_client(), editor.stringify(), gs);
                    }
                    entry.insert(lsp);
                }
            }
            Entry::Occupied(entry) => {
                new.lexer.set_lsp_client(entry.get().aquire_client(), new.stringify(), gs);
            }
        }
        Ok(new)
    }

    pub async fn new_from(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<bool> {
        let file_path = file_path.canonicalize()?;
        if let Some(idx) = self.editors.iter().position(|e| e.path == file_path) {
            let mut editor = self.editors.remove(idx);
            editor.clear_screen_cache();
            self.editors.insert(0, editor);
            return Ok(false);
        }
        let editor = self.build_editor(file_path, gs).await?;
        self.editors.insert(0, editor);
        self.toggle_editor();
        Ok(true)
    }

    pub async fn new_at_line(&mut self, file_path: PathBuf, line: usize, gs: &mut GlobalState) -> IdiomResult<()> {
        if self.new_from(file_path, gs).await? {
            if let Some(editor) = self.get_active() {
                editor.go_to(line);
            }
        };
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

    #[inline]
    pub async fn check_lsp(&mut self, ft: FileType, gs: &mut GlobalState) {
        if let Some(lsp) = self.lsp_servers.get_mut(&ft) {
            match lsp.check_status().await {
                Ok(data) => match data {
                    None => gs.success("LSP function is normal".to_owned()),
                    Some(err) => {
                        self.full_sync(&ft, gs);
                        gs.success(format!("LSP recoved after: {err}"));
                    }
                },
                Err(err) => gs.error(err.to_string()),
            }
        }
    }

    #[inline]
    pub fn full_sync(&mut self, ft: &FileType, gs: &mut GlobalState) {
        if let Some(lsp) = self.lsp_servers.get(ft) {
            for editor in self.editors.iter_mut().filter(|e| &e.file_type == ft) {
                editor.lexer.set_lsp_client(lsp.aquire_client(), editor.stringify(), gs);
            }
        }
    }

    pub fn close_active(&mut self, gs: &mut GlobalState) {
        if self.editors.is_empty() {
            return;
        }
        self.editors.remove(0);
        if self.editors.is_empty() {
            gs.editor_area.clear(&mut gs.writer);
            gs.select_mode();
        } else {
            self.editors.inner_mut_no_update()[0].clear_screen_cache();
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

    pub fn go_to_tab(&mut self, idx: usize, gs: &mut GlobalState) {
        if self.editors.is_empty() {
            return;
        }
        if idx == 0 || self.editors.len() == 1 {
            self.toggle_editor();
            gs.insert_mode();
            return;
        }
        let mut editor =
            if idx >= self.editors.len() { self.editors.pop().expect("garded") } else { self.editors.remove(idx) };
        gs.tree.push(TreeEvent::SelectPath(editor.path.clone()));
        editor.clear_screen_cache();
        self.editors.insert(0, editor);
        self.toggle_editor();
        gs.insert_mode();
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

    pub fn refresh_cfg(&mut self, new_key_map: EditorKeyMap, gs: &mut GlobalState) {
        self.key_map = new_key_map;
        gs.unwrap_or_default(self.base_config.refresh(), ".config: ");
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_config);
            editor.lexer.reload_theme(gs);
            if let Some(lsp) = self.lsp_servers.get(&editor.file_type) {
                if !editor.lexer.lsp {
                    editor.lexer.set_lsp_client(lsp.aquire_client(), editor.stringify(), gs);
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
        let (taken, render_update) = editor.lexer.map_modal_if_exists(key, gs);
        if let Some(modal_rect) = render_update {
            editor.updated_rect(modal_rect, gs);
        }
        if taken {
            return true;
        };
        if let Some(action) = action {
            if !editor.map(action, gs) {
                match action {
                    EditorAction::Close => ws.close_active(gs),
                    EditorAction::Cancel if ws.editors.len() > 1 => ws.toggle_tabs(),
                    _ => return false,
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
                ws.editors.inner_mut_no_update()[0].clear_screen_cache();
                gs.tree.push(TreeEvent::SelectPath(ws.editors.inner()[0].path.clone()));
            }
            EditorAction::Left | EditorAction::Unintent => {
                if let Some(mut editor) = ws.editors.pop() {
                    gs.tree.push(TreeEvent::SelectPath(editor.path.clone()));
                    editor.clear_screen_cache();
                    ws.editors.insert(0, editor);
                }
            }
            EditorAction::Cancel => {
                ws.toggle_editor();
                return false;
            }
            EditorAction::Close => {
                ws.close_active(gs);
            }
            _ => (),
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
