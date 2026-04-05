mod modes;
use crate::{
    configs::{EditorConfigs, EditorKeyMap, FileFamily, FileType},
    cursor::Cursor,
    editor::{Editor, editor_from_data},
    editor_line::EditorLine,
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    lsp::servers::{InitCfg, LSPRunningStatus, LSPServerStatus, LSPServers},
    popups::popups_editor::file_updated,
    utils::TrackedList,
};
use crossterm::event::KeyEvent;
use crossterm::style::{Attribute, Attributes, Color, ContentStyle};
use idiom_tui::layout::Rect;
use lsp_types::{DocumentChangeOperation, DocumentChanges, OneOf, ResourceOp, TextDocumentEdit, WorkspaceEdit};
use modes::Mode;
use std::path::PathBuf;

pub const FILE_STATUS_ERR: &str = "File status ERR";
pub const TAB_SELECT: Color = Color::DarkYellow;
const DEFAULT_TAB_STYLE: ContentStyle = ContentStyle {
    foreground_color: None,
    background_color: None,
    underline_color: None,
    attributes: Attributes::none().with(Attribute::Underlined),
};

/// implement Drop to attempt keep state upon close/crash
pub struct Workspace {
    editors: TrackedList<Editor>,
    base_configs: EditorConfigs,
    key_map: EditorKeyMap,
    lsp_servers: LSPServers,
    mode: Mode,
}

impl Workspace {
    pub fn new(
        key_map: EditorKeyMap,
        base_configs: EditorConfigs,
        lsp_preloads: Vec<(FileType, String, InitCfg)>,
    ) -> Self {
        let lsp_servers = LSPServers::new(lsp_preloads);
        Self { editors: TrackedList::new(), base_configs, key_map, lsp_servers, mode: Mode::new_editor() }
    }

    pub fn render(&mut self, gs: &mut GlobalState) {
        let mut editors = self.editors.iter();
        let Some(line) = gs.tab_area().into_iter().next() else {
            return;
        };
        let Some(editor) = editors.next() else {
            line.render_empty(&mut gs.backend);
            return;
        };
        let mut builder = line.unsafe_builder(&mut gs.backend);
        builder.push("[ ");
        builder.push_styled(editor.name(), self.mode.style());
        builder.push(saved_mark(editor));
        if editors.all(|e| {
            builder.push("| ") && builder.push_styled(e.name(), DEFAULT_TAB_STYLE) && builder.push(saved_mark(e))
        }) {
            builder.push("]");
            builder.pad_styled(DEFAULT_TAB_STYLE);
        }
    }

    pub fn fast_render(&mut self, gs: &mut GlobalState) {
        if self.editors.collect_status() {
            self.render(gs);
        }
    }

    pub fn map(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        Mode::map(self, key, gs)
    }

    /// mode handles

    #[inline]
    pub fn is_toggled_tabs(&self) -> bool {
        self.mode.is_tab()
    }

    #[inline]
    pub fn is_toggled_editor(&self) -> bool {
        self.mode.is_editor()
    }

    pub fn toggle_tabs(&mut self) {
        self.mode.to_tab();
        self.editors.mark_updated();
    }

    pub fn toggle_editor(&mut self) {
        self.mode.to_editor();
        self.editors.mark_updated();
    }

    #[inline]
    pub fn resize_all(&mut self, editor_area: Rect) {
        for editor in self.editors.iter_mut() {
            editor.resize(editor_area.width, editor_area.height as usize);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.editors.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Editor> {
        self.editors.iter()
    }

    pub fn tabs(&self) -> Vec<String> {
        self.editors.iter().map(|editor| editor.name().to_owned()).collect()
    }

    #[inline(always)]
    pub fn get_active(&mut self) -> Option<&mut Editor> {
        self.editors.get_mut_no_update(0)
    }

    #[inline]
    pub fn save_active(&mut self, gs: &mut GlobalState) {
        let Some(editor) = self.get_active() else { return };
        editor.save(gs);
        self.toggle_editor();
    }

    #[inline]
    pub fn rename_editors(&mut self, from_path: PathBuf, to_path: PathBuf, gs: &mut GlobalState) {
        if to_path.is_dir() {
            for editor in self.editors.iter_mut().filter(|e| e.path().starts_with(&from_path)) {
                let mut updated_path = PathBuf::new();
                let mut old = editor.path().iter();
                for (new_part, ..) in to_path.iter().zip(&mut old) {
                    updated_path.push(new_part);
                }
                for remaining_part in old {
                    updated_path.push(remaining_part)
                }
                gs.log_if_lsp_error(editor.update_path(updated_path), *editor.file_type());
            }
        } else if let Some(editor) = self.editors.find(|e| e.path() == &from_path) {
            gs.log_if_lsp_error(editor.update_path(to_path), *editor.file_type());
        }
    }

    pub fn activate_editor(&mut self, idx: usize, gs: &mut GlobalState) -> Option<&mut Editor> {
        if idx >= self.editors.len() {
            return None;
        }
        let mut editor = self.editors.remove(idx);
        editor.clear_screen_cache(gs);
        gs.select_editor_events(&editor);
        Some(self.editors.insert_and_get_mut(0, editor))
    }

    pub fn apply_edits(&mut self, edits: WorkspaceEdit, gs: &mut GlobalState) {
        if let Some(edits) = edits.changes {
            for (file_url, file_edits) in edits {
                if let Some(editor) = self.get_editor(file_url.path().as_str()) {
                    editor.apply_file_edits(file_edits);
                } else if let Ok(mut editor) = self.build_basic_editor(PathBuf::from(file_url.path().as_str()), gs) {
                    editor.apply_file_edits(file_edits);
                    editor.write_file_logged(gs);
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
            editor.write_file_logged(gs);
        } else {
            gs.error(format!("Unable to build editor for {}", text_document_edit.text_document.uri.path()));
        };
    }

    fn handle_tree_operations(&mut self, operation: ResourceOp) -> IdiomResult<()> {
        match operation {
            ResourceOp::Create(create) => {
                let path = PathBuf::from(create.uri.path().as_str());
                if path.exists()
                    && let Some(options) = create.options
                    && (matches!(options.overwrite, Some(overwrite) if !overwrite)
                        || matches!(options.ignore_if_exists, Some(ignore) if ignore))
                {
                    return Err(IdiomError::io_exists(format!("File {path:?} already exists!")));
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
                    let new_path = PathBuf::from(rename.new_uri.path().as_str());
                    editor.update_path(new_path)?;
                }
            }
        }
        Ok(())
    }

    fn get_editor<T: Into<PathBuf>>(&mut self, path: T) -> Option<&mut Editor> {
        let path: PathBuf = path.into();
        self.editors.iter_mut().find(|editor| editor.path() == &path)
    }

    // EDITOR BUILDERS

    pub fn create_text_editor_from_data(
        &mut self,
        path: PathBuf,
        content: Vec<EditorLine>,
        cursor: Option<Cursor>,
        gs: &mut GlobalState,
    ) {
        self.editors.insert(0, editor_from_data(path, FileType::Text, content, cursor, &self.base_configs, gs));
    }

    /// it could be the case that the file no longer exits
    pub fn create_editor_from_session(
        &mut self,
        path: PathBuf,
        file_type: FileType,
        cursor: Cursor,
        content: Option<Vec<String>>,
        gs: &mut GlobalState,
    ) -> IdiomResult<()> {
        let content = match content {
            None => EditorLine::parse_lines_raw(&path)?,
            Some(lines) => lines.into_iter().map(EditorLine::from).collect(),
        };
        let mut editor = editor_from_data(path, file_type, content, Some(cursor), &self.base_configs, gs);
        if editor.file_type().is_code() {
            lsp_enroll(&mut editor, &mut self.lsp_servers, &self.base_configs, gs);
        }
        if editor.path().exists() {
            match editor.is_saved_content_update() {
                // the check will update it
                Ok(true) => (),
                Ok(false) => editor.file_status_set_overwritten(),
                Err(error) => gs.error(error),
            }
        }
        self.editors.insert(0, editor);
        Ok(())
    }

    pub fn get_or_create_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<&mut Editor> {
        let file_path = file_path.canonicalize()?;
        let editor = match self.editors.remove_if(|e| e.path() == &file_path) {
            Some(mut editor) => {
                editor.clear_screen_cache(gs);
                if editor.file_status_is_update() {
                    gs.event.push(file_updated(editor.path().to_owned()).into());
                }
                editor
            }
            None => self.build_editor(file_path, gs)?,
        };
        self.toggle_editor();
        Ok(self.editors.insert_and_get_mut(0, editor))
    }

    fn build_basic_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<Editor> {
        Editor::from_path(file_path, FileType::Text, &self.base_configs, gs)
    }

    fn build_editor(&mut self, file_path: PathBuf, gs: &mut GlobalState) -> IdiomResult<Editor> {
        match FileFamily::derive_type(&file_path) {
            FileFamily::Text => Editor::from_path_text(file_path, &self.base_configs, gs),
            FileFamily::MarkDown => Editor::from_path_md(file_path, &self.base_configs, gs),
            FileFamily::Code(file_type) => {
                let mut editor = Editor::from_path(file_path, file_type, &self.base_configs, gs)?;
                lsp_enroll(&mut editor, &mut self.lsp_servers, &self.base_configs, gs);
                Ok(editor)
            }
        }
    }

    // LSP HANDLES

    pub fn connect_ready_lsp_servs(&mut self, gs: &mut GlobalState) {
        if self.lsp_servers.are_all_servers_ready() {
            return;
        }
        self.lsp_servers.apply_started_servers(|file_type, lsp_result| match lsp_result {
            Ok(lsp) => {
                for editor in self.editors.iter_mut().filter(|e| e.file_type() == &file_type) {
                    editor.lsp_set(lsp.aquire_client(), gs);
                }
            }
            Err(error) => {
                gs.error(error);
                for editor in self.editors.iter_mut().filter(|e| e.file_type() == &file_type) {
                    editor.lsp_local(gs);
                }
            }
        });
    }

    pub fn force_lsp_type_on_active(&mut self, file_type: FileType, gs: &mut GlobalState) -> IdiomResult<()> {
        let new_indent_cfg = self.base_configs.get_indent_cfg(file_type);
        let Some(editor) = self.editors.get_mut_no_update(0) else {
            return Err(IdiomError::LSP(crate::lsp::LSPError::Null));
        };

        editor.file_type_set(file_type, new_indent_cfg, gs);

        if file_type.is_code() {
            lsp_enroll(editor, &mut self.lsp_servers, &self.base_configs, gs);
        }

        Ok(())
    }

    #[inline]
    pub fn check_lsp(&mut self, file_type: FileType, gs: &mut GlobalState) {
        let Some(status) = self.lsp_servers.check_running_lsp(file_type, &self.base_configs) else {
            return;
        };
        match status {
            LSPRunningStatus::Running => gs.success("LSP function is normal".to_owned()),
            LSPRunningStatus::Dead => {
                gs.error(format!("LSP ({}) failed recovery >> moving to local LSP!", file_type.as_str()));
                for editor in self.editors.iter_mut().filter(|e| e.file_type() == &file_type) {
                    editor.lsp_local(gs);
                }
            }
            LSPRunningStatus::Failing => {
                gs.error(format!("LSP ({}) is failing >> attemping to recover ...", file_type.as_str()));
                for editor in self.editors.iter_mut().filter(|e| e.file_type() == &file_type) {
                    editor.lsp_drop();
                }
            }
        }
    }

    pub fn notify_update(&mut self, path: PathBuf, gs: &mut GlobalState) {
        for (idx, editor) in self.editors.iter_mut().enumerate() {
            if editor.path() == &path {
                let save_status_result = editor.is_saved_content_update();
                if gs.unwrap_or_default(save_status_result, FILE_STATUS_ERR) {
                    return;
                }
                editor.file_status_set_overwritten();
                if idx == 0 && editor.file_status_is_update() {
                    gs.event.push(file_updated(path).into());
                }
                return;
            }
        }
    }

    pub fn select_tab_mouse(&mut self, col_idx: usize, gs: &mut GlobalState) -> Option<&mut Editor> {
        let mut cols_len = 0;
        for (editor_idx, editor) in self.editors.iter().enumerate() {
            cols_len += editor.name().len() + 3;
            if col_idx < cols_len {
                return self.activate_editor(editor_idx, gs);
            };
        }
        None
    }

    pub fn is_close_tab_mouse(&mut self, col_idx: usize, gs: &mut GlobalState) -> bool {
        let mut cols_len = 0;
        for (editor_idx, editor) in self.editors.iter().enumerate() {
            cols_len += editor.name().len() + 3;
            if col_idx < cols_len {
                let editor = self.editors.remove(editor_idx);
                drop(editor);
                if editor_idx == 0
                    && let Some(editor) = self.get_active()
                {
                    editor.clear_screen_cache(gs);
                } else if self.editors.is_empty() {
                    gs.select_mode_no_editor();
                }
                return true;
            };
        }
        false
    }

    pub fn close_active(&mut self, gs: &mut GlobalState) {
        if self.editors.is_empty() {
            return;
        }
        let editor = self.editors.remove(0);
        drop(editor);
        match self.get_active() {
            None => gs.select_mode_no_editor(),
            Some(editor) => {
                editor.clear_screen_cache(gs);
                gs.select_editor_events(editor);
            }
        }
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

        let mut editor = self.editors.remove(std::cmp::min(idx, self.editors.len() - 1));
        editor.clear_screen_cache(gs);
        gs.select_editor_events(&editor);
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
        self.base_configs = gs.reload_confgs();
        for editor in self.editors.iter_mut() {
            editor.refresh_cfg(&self.base_configs, gs);
            if editor.lexer().is_external_lsp() {
                continue;
            }
            if let Some(lsp) = self.lsp_servers.get_running(editor.file_type()) {
                editor.lsp_set(lsp.aquire_client(), gs);
            }
        }
        &mut self.base_configs
    }
}

/// helper to match behavior on all lsp application
fn lsp_enroll(editor: &mut Editor, lsp_servers: &mut LSPServers, cfg: &EditorConfigs, gs: &mut GlobalState) {
    match lsp_servers.get_or_init_server(editor.file_type(), cfg) {
        LSPServerStatus::ReadyClient(client) => editor.lsp_set(*client, gs),
        LSPServerStatus::None => editor.lsp_local(gs),
        LSPServerStatus::Pending => editor.force_local_lsp_tokens(gs),
    }
}

#[inline]
fn saved_mark(editor: &Editor) -> &str {
    match editor.is_saved() {
        true => " ",
        false => "*",
    }
}

#[cfg(test)]
pub mod tests;
