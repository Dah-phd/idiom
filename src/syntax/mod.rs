mod line_builder;
mod modal;
mod theme;
use crate::{
    configs::FileType,
    global_state::{GlobalState, WorkspaceEvent},
    lsp::LSPClient,
    popups::popups_tree::refrence_selector,
    workspace::actions::EditMetaData,
    workspace::cursor::Cursor,
    workspace::CursorPosition,
};
use crossterm::event::KeyEvent;
use line_builder::LineBuilder;
use lsp_types::{PublishDiagnosticsParams, TextDocumentContentChangeEvent};
use modal::{LSPModal, LSPResponseType, LSPResult, ModalMessage};
use ratatui::{layout::Rect, widgets::ListItem, Frame};
use std::path::{Path, PathBuf};
use theme::Theme;

#[cfg(build = "debug")]
use crate::utils::debug_to_file;

pub struct Lexer {
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp_client: Option<LSPClient>,
    pub line_number_offset: usize,
    pub path: PathBuf,
    pub line_builder: LineBuilder,
    modal: Option<LSPModal>,
    requests: Vec<LSPResponseType>,
}

impl Lexer {
    pub fn with_context(file_type: FileType, path: &Path) -> Self {
        Self {
            line_builder: LineBuilder::new(file_type.into()),
            modal: None,
            path: path.into(),
            requests: Vec::new(),
            line_number_offset: 0,
            diagnostics: None,
            lsp_client: None,
        }
    }

    pub fn context(&mut self, cursor: &Cursor, content: &[String], gs: &mut GlobalState) {
        self.line_builder.reset(cursor);
        self.line_number_offset = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
        if let Some(client) = self.lsp_client.as_mut() {
            // diagnostics
            if let Some(params) = client.get_diagnostics(&self.path) {
                self.line_builder.set_diganostics(params);
            }
            // responses
            let mut unresolved_requests = Vec::new();
            for request in self.requests.drain(..) {
                if let Some(response) = client.get(request.id()) {
                    match request.parse(response.result) {
                        Some(result) => match result {
                            LSPResult::Completion(completions, line, idx) => {
                                self.modal = LSPModal::auto_complete(completions, line, idx);
                            }
                            LSPResult::Hover(hover) => {
                                if let Some(modal) = self.modal.as_mut() {
                                    modal.hover_map(hover);
                                } else {
                                    self.modal.replace(hover.into());
                                }
                            }
                            LSPResult::SignatureHelp(signature) => {
                                if let Some(modal) = self.modal.as_mut() {
                                    modal.signature_map(signature);
                                } else {
                                    self.modal.replace(signature.into());
                                }
                            }
                            LSPResult::Renames(workspace_edit) => {
                                gs.workspace.push_back(workspace_edit.into());
                            }
                            LSPResult::Tokens(tokens) => {
                                if self.line_builder.set_tokens(tokens) {
                                    gs.success("LSP tokens mapped!");
                                } else if let Some(id) = client.request_full_tokens(&self.path) {
                                    unresolved_requests.push(LSPResponseType::Tokens(id));
                                };
                            }
                            LSPResult::TokensPartial(tokens) => {
                                self.line_builder.set_tokens_partial(tokens);
                            }
                            LSPResult::References(locations) => {
                                if let Some(mut locations) = locations {
                                    if locations.len() == 1 {
                                        gs.tree.push(locations.remove(0).into());
                                    } else {
                                        gs.popup(refrence_selector(locations));
                                    }
                                }
                            }
                            LSPResult::Declaration(declaration) => {
                                gs.try_tree_event(declaration);
                            }
                            LSPResult::Definition(definition) => {
                                gs.try_tree_event(definition);
                            }
                        },
                        None => {
                            if let Some(err) = response.error {
                                gs.error(err.to_string());
                            }
                        }
                    }
                } else {
                    unresolved_requests.push(request);
                }
            }
            self.requests = unresolved_requests;
        }
    }

    pub fn set_text_width(&mut self, width: usize) {
        self.line_builder.text_width = width.checked_sub(self.line_number_offset).unwrap_or_default();
    }

    pub fn sync_lsp(
        &mut self,
        version: i32,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[String],
    ) {
        if let Some(client) = self.lsp_client.as_mut() {
            if let Some(request) = self.line_builder.collect_changes(&self.path, version, events, content, client) {
                self.requests.push(request);
            }
        } else {
            events.clear();
        }
    }

    pub fn render_modal_if_exist(&mut self, frame: &mut Frame, area: Rect, cursor: &Cursor) {
        if let Some(modal) = &mut self.modal {
            let cursor_x_offset = 1 + cursor.char;
            let cursor_y_offset = cursor.line - cursor.at_line;
            let x = area.x + (cursor_x_offset + self.line_number_offset) as u16;
            let y = area.y + cursor_y_offset as u16;
            modal.render_at(frame, x, y);
        }
    }

    pub fn map_modal_if_exists(&mut self, key: &KeyEvent, gs: &mut GlobalState) -> bool {
        if let Some(modal) = &mut self.modal {
            match modal.map_and_finish(key, gs) {
                ModalMessage::Taken => return true,
                ModalMessage::TakenDone => {
                    self.modal.take();
                    return true;
                }
                ModalMessage::Done => {
                    self.modal.take();
                }
                ModalMessage::RenameVar(new_name, c) => {
                    self.get_rename(c, new_name);
                    self.modal.take();
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    pub fn set_lsp_client(
        &mut self,
        mut client: LSPClient,
        file_type: &FileType,
        content: String,
        gs: &mut GlobalState,
    ) {
        gs.message("Mapping LSP ...");
        if client.file_did_open(&self.path, file_type, content).is_err() {
            return;
        }
        self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
        gs.success("LSP mapped!");
        if let Some(id) = client.request_full_tokens(&self.path) {
            self.requests.push(LSPResponseType::Tokens(id));
            gs.message("Getting LSP semantic tokents ...");
        };
        self.lsp_client.replace(client);
    }

    pub fn should_autocomplete(&mut self, char_idx: usize, line: &str) -> bool {
        self.lsp_client.is_some()
            && self.line_builder.lang.completelable(line, char_idx)
            && !matches!(self.modal, Some(LSPModal::AutoComplete(..)))
    }

    pub fn get_autocomplete(&mut self, c: CursorPosition, line: &str) {
        if let Some(id) = self.lsp_client.as_mut().and_then(|client| client.request_completions(&self.path, c)) {
            self.requests.push(LSPResponseType::Completion(id, line.to_owned(), c.char));
        }
    }

    pub fn start_rename(&mut self, c: CursorPosition, title: &str) {
        if let Some(client) = self.lsp_client.as_mut() {
            if client.capabilities.rename_provider.is_none() {
                return;
            }
        }
        self.modal.replace(LSPModal::renames_at(c, title));
    }

    pub fn help(&mut self, c: CursorPosition) {
        if let Some(client) = self.lsp_client.as_mut() {
            if let Some(actions) = self.line_builder.collect_actions(c.line) {
                self.modal.replace(LSPModal::actions(actions));
            }
            if let Some(id) = client.request_signitures(&self.path, c).map(LSPResponseType::SignatureHelp) {
                self.requests.push(id);
            }
            if let Some(id) = client.request_hover(&self.path, c).map(LSPResponseType::Hover) {
                self.requests.push(id);
            }
        }
    }

    pub fn get_rename(&mut self, c: CursorPosition, new_name: String) {
        if let Some(id) = self
            .lsp_client
            .as_mut()
            .and_then(|client| client.request_rename(&self.path, c, new_name))
            .map(LSPResponseType::Renames)
        {
            self.requests.push(id);
        }
    }

    pub fn go_to_declaration(&mut self, c: CursorPosition) {
        if let Some(id) = self
            .lsp_client
            .as_mut()
            .and_then(|client| client.request_declarations(&self.path, c))
            .map(LSPResponseType::Declaration)
        {
            self.requests.push(id);
        }
    }

    pub fn go_to_reference(&mut self, c: CursorPosition) {
        if let Some(id) = self
            .lsp_client
            .as_mut()
            .and_then(|client| client.request_references(&self.path, c))
            .map(LSPResponseType::References)
        {
            self.requests.push(id);
        }
    }

    pub fn list_item<'a>(&mut self, line_idx: usize, content: &'a str) -> ListItem<'a> {
        self.line_builder.build_line(line_idx, content, self.line_number_offset)
    }

    pub fn reload_theme(&mut self) {
        self.line_builder.theme = Theme::new();
        if let Some(client) = self.lsp_client.as_mut() {
            self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
        }
    }

    pub fn save_and_check_lsp(&mut self, file_type: FileType, gs: &mut GlobalState) {
        self.line_builder.mark_saved();
        if let Some(client) = self.lsp_client.as_mut() {
            gs.message("Checking LSP status (on save) ...");
            if client.file_did_save(&self.path).is_err() && client.is_closed() {
                gs.workspace.push_back(WorkspaceEvent::CheckLSP(file_type));
            } else {
                gs.success("LSP running ...");
            }
            if let Some(id) = client.request_full_tokens(&self.path) {
                self.requests.push(LSPResponseType::Tokens(id));
            }
        }
    }
}
