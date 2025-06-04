pub mod actions;
pub mod cursor;
pub mod editor;
pub mod line;
pub mod renderer;
pub mod utils;
use crate::popups::popups_editor::file_updated;
use crate::{
    configs::{EditorAction, EditorConfigs, EditorKeyMap, FileType},
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, IdiomEvent},
    lsp::LSP,
    utils::TrackedList,
};
use crossterm::event::KeyEvent;
use crossterm::style::{Color, ContentStyle};
pub use cursor::CursorPosition;
pub use editor::{editor_from_data, Editor};
use idiom_ui::backend::{Backend, StyleExt};
use line::EditorLine;
use lsp_types::{DocumentChangeOperation, DocumentChanges, OneOf, ResourceOp, TextDocumentEdit, WorkspaceEdit};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
};

const FILE_STATUS_ERR: &str = "File status ERR";

/// implement Drop to attempt keep state upon close/crash
pub struct Workspace {
    editors: TrackedList<Editor>,
    base_configs: EditorConfigs,
    key_map: EditorKeyMap,
    tab_style: ContentStyle,
    lsp_servers: HashMap<FileType, LSP>,
    map_callback: fn(&mut Self, &KeyEvent, &mut GlobalState) -> bool,
}

impl Workspace {
    pub async fn new(key_map: EditorKeyMap, base_configs: EditorConfigs, lsp_servers: HashMap<FileType, LSP>) -> Self {
        let tab_style = ContentStyle::fg(Color::DarkYellow);
        Self { editors: TrackedList::new(), base_configs, key_map, lsp_servers, map_callback: map_editor, tab_style }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        if let Some(editor) = self.editors.get_mut(0) {
            let line = match gs.tab_area.into_iter().next() {
                Some(line) => line,
                None => return,
            };
            gs.backend.set_style(ContentStyle::underlined(None));
            {
                let mut builder = line.unsafe_builder(&mut gs.backend);
                builder.push_styled(&editor.display, self.tab_style);
                for editor in self.editors.iter().skip(1) {
                    if !builder.push(" | ") || !builder.push(&editor.display) {
                        break;
                    };
                }
            }
            gs.backend.reset_style();
        } else if let Some(line) = gs.tab_area.into_iter().next() {
            line.render_empty(&mut gs.backend);
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
        self.tab_style = ContentStyle::reversed();
    }

    pub fn toggle_editor(&mut self) {
        self.editors.mark_updated();
        self.map_callback = map_editor;
        self.tab_style = ContentStyle::fg(Color::DarkYellow);
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
    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut_no_update(0)
    }

    #[inline]
    pub fn rename_editors(&mut self, from_path: PathBuf, to_path: PathBuf, gs: &mut GlobalState) {
        if to_path.is_dir() {
            for editor in self.editors.iter_mut() {
                if editor.path.starts_with(&from_path) {
                    let mut updated_path = PathBuf::new();
                    let mut old = editor.path.iter();
                    for (new_part, ..) in to_path.iter().zip(&mut old) {
                        updated_path.push(new_part);
                    }
                    for remaining_part in old {
                        updated_path.push(remaining_part)
                    }
                    gs.log_if_lsp_error(editor.update_path(updated_path), editor.file_type);
                }
            }
        } else if let Some(editor) = self.editors.find(|e| e.path == from_path) {
            gs.log_if_lsp_error(editor.update_path(to_path), editor.file_type);
        }
    }

    pub fn activate_editor(&mut self, idx: usize, gs: &mut GlobalState) {
        if idx < self.editors.len() {
            let mut editor = self.editors.remove(idx);
            editor.clear_screen_cache(gs);
            gs.event.push(IdiomEvent::SelectPath(editor.path.clone()));
            if editor.update_status.collect() {
                gs.event.push(file_updated(editor.path.clone()).into());
            }
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
                            return Err(IdiomError::io_exists(format!("File {path:?} already exists!")));
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

    fn get_editor<T: Into<PathBuf>>(&mut self, path: T) -> Option<&mut Editor> {
        let path: PathBuf = path.into();
        self.editors.iter_mut().find(|editor| editor.path == path)
    }

    fn build_basic_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<Editor> {
        Editor::from_path(file_path, FileType::Ignored, &self.base_configs, gs)
    }

    async fn build_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<Editor> {
        let file_type = match FileType::derive_type(&file_path) {
            Some(file_type) => file_type,
            None => {
                return match file_path.extension().and_then(|ext| ext.to_str()) {
                    Some(ext) if ext.to_lowercase() == "md" => Editor::from_path_md(file_path, &self.base_configs, gs),
                    _ => Editor::from_path_text(file_path, &self.base_configs, gs),
                }
            }
        };
        let mut new = Editor::from_path(file_path, file_type, &self.base_configs, gs)?;
        let lsp_cmd = match self.base_configs.derive_lsp(&new.file_type) {
            None => {
                new.lexer.local_lsp(file_type, new.stringify(), gs);
                return Ok(new);
            }
            Some(cmd) => cmd,
        };

        // set initial tokens while LSP is indexing
        crate::lsp::init_local_tokens(file_type, &mut new.content, &new.lexer.theme);
        match self.lsp_servers.entry(new.file_type) {
            Entry::Vacant(entry) => match LSP::new(lsp_cmd, new.file_type).await {
                Ok(lsp) => {
                    let client = lsp.aquire_client();
                    new.lexer.set_lsp_client(client, new.stringify(), gs);
                    for editor in self.editors.iter_mut().filter(|e| e.file_type == new.file_type) {
                        editor.lexer.set_lsp_client(lsp.aquire_client(), editor.stringify(), gs);
                    }
                    entry.insert(lsp);
                }
                Err(err) => {
                    gs.error(err.to_string());
                    new.lexer.local_lsp(file_type, new.stringify(), gs);
                }
            },
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
            editor.clear_screen_cache(gs);
            if editor.update_status.collect() {
                gs.event.push(file_updated(editor.path.clone()).into());
            }
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
            match lsp.check_status(ft).await {
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

    pub fn notify_update(&mut self, path: PathBuf, gs: &mut GlobalState) {
        for (idx, editor) in self.editors.iter_mut().enumerate() {
            if editor.path == path {
                let save_status_result = editor.is_saved();
                if gs.unwrap_or_default(save_status_result, FILE_STATUS_ERR) {
                    return;
                }
                editor.update_status.mark_updated();
                if idx == 0 && editor.update_status.collect() {
                    gs.event.push(file_updated(path).into());
                }
                return;
            }
        }
    }

    pub fn close_active(&mut self, gs: &mut GlobalState) {
        if self.editors.is_empty() {
            return;
        }
        let editor = self.editors.remove(0);
        drop(editor);
        match self.get_active() {
            None => {
                gs.clear_stats();
                gs.editor_area.clear(&mut gs.backend);
                gs.select_mode();
            }
            Some(editor) => {
                editor.clear_screen_cache(gs);
                if editor.update_status.collect() {
                    gs.event.push(file_updated(editor.path.clone()).into());
                }
            }
        }
    }

    pub fn are_updates_saved(&self, gs: &mut GlobalState) -> bool {
        for editor in self.editors.iter() {
            let save_status_result = editor.is_saved();
            if !gs.unwrap_or_default(save_status_result, FILE_STATUS_ERR) {
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
        gs.event.push(IdiomEvent::SelectPath(editor.path.clone()));
        editor.clear_screen_cache(gs);
        if editor.update_status.collect() {
            gs.event.push(file_updated(editor.path.clone()).into());
        }
        self.editors.insert(0, editor);
        self.toggle_editor();
        gs.insert_mode();
    }

    pub fn save_all(&mut self, gs: &mut GlobalState) {
        for editor in self.editors.iter_mut() {
            editor.save(gs);
        }
    }

    pub fn refresh_cfg(&mut self, new_editor_key_map: EditorKeyMap, gs: &mut GlobalState) -> &mut EditorConfigs {
        self.key_map = new_editor_key_map;
        gs.unwrap_or_default(self.base_configs.refresh(), ".config: ");
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_configs);
            editor.lexer.reload_theme(gs);
            if let Some(lsp) = self.lsp_servers.get(&editor.file_type) {
                if !editor.lexer.lsp {
                    editor.lexer.set_lsp_client(lsp.aquire_client(), editor.stringify(), gs);
                }
            }
        }
        &mut self.base_configs
    }
}

/// handels keybindings for editor
fn map_editor(ws: &mut Workspace, key: &KeyEvent, gs: &mut GlobalState) -> bool {
    let editor = match ws.editors.get_mut_no_update(0) {
        None => return false,
        Some(editor) => editor,
    };
    let action = match ws.key_map.map(key) {
        None => return false,
        Some(action) => action,
    };
    if !editor.map(action, gs) {
        match action {
            EditorAction::Close => ws.close_active(gs),
            EditorAction::Cancel if ws.editors.len() > 1 => ws.toggle_tabs(),
            _ => return false,
        }
    }
    true
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
                let editor = &mut ws.editors.inner_mut_no_update()[0];
                editor.clear_screen_cache(gs);
                if editor.update_status.collect() {
                    gs.event.push(file_updated(editor.path.clone()).into());
                }
                gs.event.push(IdiomEvent::SelectPath(ws.editors.inner()[0].path.clone()));
            }
            EditorAction::Left | EditorAction::Unintent => {
                if let Some(mut editor) = ws.editors.pop() {
                    gs.event.push(IdiomEvent::SelectPath(editor.path.clone()));
                    editor.clear_screen_cache(gs);
                    if editor.update_status.collect() {
                        gs.event.push(file_updated(editor.path.clone()).into());
                    }
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

pub fn add_editor_from_data(
    workspace: &mut Workspace,
    path: PathBuf,
    content: Vec<EditorLine>,
    file_type: FileType,
    gs: &mut GlobalState,
) {
    let editor = editor_from_data(path, content, file_type, &workspace.base_configs, gs);
    workspace.editors.insert(0, editor);
}

#[cfg(test)]
pub mod tests;
