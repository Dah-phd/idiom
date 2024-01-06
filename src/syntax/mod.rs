mod line_builder;
mod modal;
mod theme;
use self::line_builder::LineBuilder;
use self::modal::{LSPModal, LSPResponseType, LSPResult, ModalMessage};
pub use self::theme::Theme;
use crate::configs::FileType;
use crate::global_state::GlobalState;
use crate::lsp::LSPClient;
use crate::popups::popups_tree::refrence_selector;
use crate::workspace::actions::EditMetaData;
use crate::workspace::cursor::Cursor;
use crate::workspace::CursorPosition;
use crossterm::event::KeyEvent;
use lsp_types::{PublishDiagnosticsParams, TextDocumentContentChangeEvent};
use ratatui::layout::Rect;
use ratatui::{widgets::ListItem, Frame};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::{fmt::Debug, path::Path};

#[cfg(build = "debug")]
use crate::utils::debug_to_file;

pub struct Lexer {
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp_client: Option<LSPClient>,
    pub max_digits: usize,
    pub path: PathBuf,
    pub line_builder: LineBuilder,
    select: Option<(CursorPosition, CursorPosition)>,
    modal: Option<LSPModal>,
    requests: Vec<LSPResponseType>,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("LEXER: {:?}", self.line_builder.lang.file_type).as_str())
    }
}

impl Lexer {
    pub fn with_context(file_type: FileType, path: &Path) -> Self {
        Self {
            line_builder: LineBuilder::new(file_type.into()),
            select: None,
            modal: None,
            path: path.into(),
            requests: Vec::new(),
            max_digits: 0,
            diagnostics: None,
            lsp_client: None,
        }
    }

    pub fn context(&mut self, cursor: &Cursor, content: &[String], gs: &mut GlobalState) {
        self.line_builder.reset(cursor.position());
        self.select = cursor.select_get();
        self.max_digits = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
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
                                    self.modal.replace(LSPModal::hover(hover));
                                }
                            }
                            LSPResult::SignatureHelp(signature) => {
                                if let Some(modal) = self.modal.as_mut() {
                                    modal.signature_map(signature);
                                } else {
                                    self.modal.replace(LSPModal::signature(signature));
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

    pub fn set_text_width(&mut self, area_width: usize) {
        self.line_builder.text_width = area_width.checked_sub(self.max_digits).unwrap_or_default();
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
            let x = area.x + (cursor_x_offset + self.max_digits) as u16;
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

    pub fn list_item<'a>(&mut self, idx: usize, content: &'a str) -> ListItem<'a> {
        self.line_builder.build_line(idx, self.line_select(idx, content.len()), content, self.max_digits)
    }

    pub fn reload_theme(&mut self) {
        self.line_builder.theme = Theme::new();
        if let Some(client) = self.lsp_client.as_mut() {
            self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
        }
    }

    pub fn save(&mut self) {
        if let Some(client) = self.lsp_client.as_mut() {
            let _ = client.file_did_save(&self.path);
            if let Some(id) = client.request_full_tokens(&self.path) {
                self.requests.push(LSPResponseType::Tokens(id));
            }
        }
        self.line_builder.mark_saved();
    }

    fn line_select(&self, at_line: usize, max_len: usize) -> Option<std::ops::Range<usize>> {
        let (from, to) = self.select?;
        match (from.line.cmp(&at_line), at_line.cmp(&to.line)) {
            (Ordering::Greater, ..) | (.., Ordering::Greater) => None,
            (Ordering::Less, Ordering::Less) => Some(0..max_len),
            (Ordering::Equal, Ordering::Equal) => Some(from.char..to.char),
            (Ordering::Equal, ..) => Some(from.char..max_len),
            (.., Ordering::Equal) => Some(0..to.char),
        }
    }
}
