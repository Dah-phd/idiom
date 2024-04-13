use ratatui::buffer::Buffer;
mod line_builder;
mod modal;
pub mod theme;
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
pub use line_builder::DiagnosticLine;
use line_builder::LineBuilder;
pub use line_builder::LineBuilderContext;
use lsp_types::{PublishDiagnosticsParams, TextDocumentContentChangeEvent};
use modal::{LSPModal, LSPResponseType, LSPResult, ModalMessage};
use ratatui::{layout::Rect, Frame};
use std::path::{Path, PathBuf};
use theme::Theme;

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
    pub fn with_context(file_type: FileType, path: &Path, content: &[String], gs: &mut GlobalState) -> Self {
        Self {
            line_builder: LineBuilder::new(file_type.into(), content, gs),
            modal: None,
            path: path.into(),
            requests: Vec::new(),
            line_number_offset: 0,
            diagnostics: None,
            lsp_client: None,
        }
    }

    pub fn context(&mut self, content: &[String], gs: &mut GlobalState) {
        self.line_number_offset = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
        if let Some(client) = self.lsp_client.as_mut() {
            // diagnostics
            if let Some(diagnostics) = client.get_diagnostics(&self.path) {
                self.line_builder.set_diganostics(diagnostics);
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
                                    modal.hover_map(hover, &self.line_builder);
                                } else {
                                    self.modal.replace(LSPModal::from_hover(hover, &self.line_builder));
                                }
                            }
                            LSPResult::SignatureHelp(signature) => {
                                if let Some(modal) = self.modal.as_mut() {
                                    modal.signature_map(signature, &self.line_builder);
                                } else {
                                    self.modal.replace(LSPModal::from_signature(signature, &self.line_builder));
                                }
                            }
                            LSPResult::Renames(workspace_edit) => {
                                gs.workspace.push(workspace_edit.into());
                            }
                            LSPResult::Tokens(tokens) => {
                                if self.line_builder.set_tokens(tokens, content) {
                                    gs.success("LSP tokens mapped!");
                                } else if let Some(id) = client.request_full_tokens(&self.path) {
                                    unresolved_requests.push(LSPResponseType::Tokens(id));
                                };
                            }
                            LSPResult::TokensPartial(tokens) => {
                                self.line_builder.set_tokens_partial(tokens, content);
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

    pub fn sync_lsp(
        &mut self,
        version: i32,
        events: &mut Vec<(EditMetaData, TextDocumentContentChangeEvent)>,
        content: &[String],
    ) {
        if let Some(request) = self
            .lsp_client
            .as_mut()
            .and_then(|client| self.line_builder.collect_changes(&self.path, version, events, content, client))
        {
            self.requests.push(request);
        } else {
            self.line_builder.update_internals(events, content);
        };
    }

    pub fn build_line(
        &mut self,
        line_idx: usize,
        text: &str,
        ctx: &mut LineBuilderContext,
        buf: &mut Buffer,
        area: Rect,
    ) {
        self.line_builder.build_line(line_idx, text, self.line_number_offset, buf, area, ctx);
    }

    pub fn build_long_line(&mut self, line_idx: usize, text: &str, buf: &mut Buffer, area: Rect) {
        self.line_builder.long_line(line_idx, text, self.line_number_offset, buf, area);
    }

    pub fn wrap_line(
        &mut self,
        line_idx: usize,
        text: &str,
        ctx: &mut LineBuilderContext,
        buf: &mut Buffer,
        max_lines: usize,
        x: u16,
    ) -> (u16, usize) {
        self.line_builder.wrap_line(line_idx, text, self.line_number_offset, buf, x, max_lines, ctx)
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
            match modal.map_and_finish(key, &self.line_builder.lang, gs) {
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
            if let Some(actions) = self.line_builder.collect_diagnostic_info(c.line) {
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

    pub fn reload_theme(&mut self, gs: &mut GlobalState) {
        self.line_builder.theme = match Theme::new() {
            Ok(theme) => theme,
            Err(err) => {
                let mut msg = "theme.json: ".to_owned();
                msg.push_str(&err.to_string());
                gs.error(msg);
                return;
            }
        };
        if let Some(client) = self.lsp_client.as_mut() {
            self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
            if let Some(id) = client.request_full_tokens(&self.path) {
                self.requests.push(LSPResponseType::Tokens(id));
            }
        };
    }

    pub fn save_and_check_lsp(&mut self, file_type: FileType, gs: &mut GlobalState) {
        self.line_builder.mark_saved();
        if let Some(client) = self.lsp_client.as_mut() {
            gs.message("Checking LSP status (on save) ...");
            if client.file_did_save(&self.path).is_err() && client.is_closed() {
                gs.workspace.push(WorkspaceEvent::CheckLSP(file_type));
            } else {
                gs.success("LSP running ...");
            }
            if let Some(id) = client.request_full_tokens(&self.path) {
                self.requests.push(LSPResponseType::Tokens(id));
            }
        }
    }
}
