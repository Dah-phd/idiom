mod line_builder;
mod modal;
mod theme;
use self::line_builder::LineBuilder;
use self::modal::{LSPModal, LSPModalResult, LSPResponseType, LSPResult};
pub use self::theme::Theme;
use crate::components::workspace::CursorPosition;
use crate::configs::EditorAction;
use crate::configs::FileType;
use crate::events::{Events, WorkspaceEvent};
use crate::lsp::{LSPClient, LSPRequest};
use lsp_types::request::{
    Completion, GotoDeclaration, GotoDefinition, HoverRequest, Rename, SemanticTokensFullRequest, SignatureHelpRequest,
};
use lsp_types::{PublishDiagnosticsParams, TextDocumentContentChangeEvent, WorkspaceEdit};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::{widgets::ListItem, Frame};
use std::cell::RefCell;
use std::fmt::Debug;
use std::{path::Path, rc::Rc};

pub struct Lexer {
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub workspace_edit: Option<WorkspaceEdit>,
    pub events: Rc<RefCell<Events>>,
    pub lsp_client: Option<LSPClient>,
    line_builder: LineBuilder,
    select: Option<(CursorPosition, CursorPosition)>,
    modal: Option<LSPModal>,
    requests: Vec<LSPResponseType>,
    max_digits: usize,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("LEXER: {:?}", self.line_builder.lang.file_type).as_str())
    }
}

impl Lexer {
    pub fn with_context(file_type: FileType, theme: Theme, events: &Rc<RefCell<Events>>) -> Self {
        Self {
            line_builder: LineBuilder::new(theme, file_type.into()),
            select: None,
            modal: None,
            requests: Vec::new(),
            max_digits: 0,
            diagnostics: None,
            workspace_edit: None,
            events: Rc::clone(events),
            lsp_client: None,
        }
    }

    pub fn context(
        &mut self,
        content: &[String],
        select: Option<(&CursorPosition, &CursorPosition)>,
        path: &Path,
    ) -> usize {
        self.get_lsp_responses();
        self.get_diagnostics(path);
        self.get_tokens(path);
        self.line_builder.reset();
        self.select = select.map(|(from, to)| (*from, *to));
        self.max_digits = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
        self.max_digits
    }

    pub async fn update_lsp(
        &mut self,
        path: &Path,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        if let Some(client) = self.lsp_client.as_mut() {
            self.line_builder.collect_changes(&content_changes);
            if let Err(err) = client.file_did_change(path, version, content_changes) {
                let mut events = self.events.borrow_mut();
                events.overwrite(format!("Failed to sync with lsp: {err}"));
                events.workspace.push(WorkspaceEvent::CheckLSP(self.line_builder.lang.file_type));
            }
        }
    }

    pub fn render_modal_if_exist(&mut self, frame: &mut Frame, x: u16, y: u16) {
        if let Some(modal) = &mut self.modal {
            modal.render_at(frame, x, y);
        }
    }

    pub fn map_modal_if_exists(&mut self, key: &EditorAction, path: &Path) -> bool {
        if let Some(modal) = &mut self.modal {
            match modal.map_and_finish(key) {
                LSPModalResult::Taken => return true,
                LSPModalResult::TakenDone => {
                    self.modal.take();
                    return true;
                }
                LSPModalResult::Done => {
                    self.modal.take();
                }
                LSPModalResult::Workspace(event) => {
                    self.events.borrow_mut().workspace.push(event);
                    self.modal.take();
                    return true;
                }
                LSPModalResult::RenameVar(new_name, c) => {
                    self.get_renames(path, &c, new_name);
                    self.modal.take();
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    pub fn set_lsp_client(&mut self, mut client: LSPClient, on_file: &Path, file_type: &FileType, content: String) {
        self.events.borrow_mut().message("Mapping LSP ...");
        if client.file_did_open(on_file, file_type, content).is_err() {
            return;
        }
        self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
        self.lsp_client.replace(client);
        self.events.borrow_mut().overwrite("LSP mapped!");
        self.get_tokens(on_file);
    }

    fn get_diagnostics(&mut self, path: &Path) -> Option<()> {
        let client = self.lsp_client.as_mut()?;
        let params = client.get_diagnostics(path)?;
        self.line_builder.set_diganostics(params);
        Some(())
    }

    pub fn start_renames(&mut self, c: &CursorPosition, title: &str) {
        if let Some(client) = self.lsp_client.as_mut() {
            if client.capabilities.rename_provider.is_none() {
                return;
            }
        }
        self.modal.replace(LSPModal::renames_at(*c, title));
    }

    pub fn get_renames(&mut self, path: &Path, c: &CursorPosition, new_name: String) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.rename_provider.as_ref()?;
        let id = self.send_request(LSPRequest::<Rename>::rename(path, c, new_name)?)?;
        self.requests.push(LSPResponseType::Renames(id));
        Some(())
    }

    pub fn get_autocomplete(&mut self, path: &Path, c: &CursorPosition, line: &str) -> Option<()> {
        if matches!(self.modal, Some(LSPModal::AutoComplete(..))) || !self.line_builder.lang.completelable(line, c.char)
        {
            return None;
        }
        let id = self.send_request(LSPRequest::<Completion>::completion(path, c)?)?;
        self.requests.push(LSPResponseType::Completion(id, line.to_owned(), c.char));
        Some(())
    }

    pub fn get_hover(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.hover_provider.as_ref()?;
        let id = self.send_request(LSPRequest::<HoverRequest>::hover(path, c)?)?;
        self.requests.push(LSPResponseType::Hover(id));
        Some(())
    }

    pub fn go_to_declaration(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.declaration_provider.as_ref()?;
        let id = self.send_request(LSPRequest::<GotoDeclaration>::declaration(path, c)?)?;
        self.requests.push(LSPResponseType::Declaration(id));
        Some(())
    }

    pub fn go_to_definition(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.definition_provider.as_ref()?;
        let id = self.send_request(LSPRequest::<GotoDefinition>::definition(path, c)?)?;
        self.requests.push(LSPResponseType::Definition(id));
        Some(())
    }

    pub fn get_signitures(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.signature_help_provider.as_ref()?;
        let id = self.send_request(LSPRequest::<SignatureHelpRequest>::signature_help(path, c)?)?;
        self.requests.push(LSPResponseType::SignatureHelp(id));
        Some(())
    }

    pub fn get_tokens(&mut self, path: &Path) -> Option<()> {
        self.lsp_client.as_ref()?.capabilities.semantic_tokens_provider.as_ref()?;
        if self.line_builder.should_update() {
            let id = self.send_request(LSPRequest::<SemanticTokensFullRequest>::semantics_full(path)?)?;
            self.requests.push(LSPResponseType::TokensFull(id));
            self.line_builder.waiting = true;
            self.events.borrow_mut().message("Getting LSP syntax");
        }
        Some(())
    }

    // error handler!
    fn send_request<T>(&mut self, request: LSPRequest<T>) -> Option<i64>
    where
        T: lsp_types::request::Request,
        T::Params: serde::Serialize,
        T::Result: serde::de::DeserializeOwned,
    {
        self.lsp_client.as_mut().as_mut()?.request(request)
    }

    fn get_lsp_responses(&mut self) -> Option<()> {
        if self.requests.is_empty() {
            return None;
        }
        let lsp = self.lsp_client.as_mut()?;
        let mut unresolved_requests = Vec::new();
        for request in self.requests.drain(..) {
            if let Some(response) = lsp.get(request.id()) {
                if let Some(value) = response.result {
                    match request.parse(value) {
                        LSPResult::Completion(completions, line, idx) => {
                            self.modal = LSPModal::auto_complete(completions, line, idx);
                        }
                        LSPResult::Hover(hover) => {
                            self.modal.replace(LSPModal::hover(hover));
                        }
                        LSPResult::SignatureHelp(signature) => {
                            self.modal.replace(LSPModal::signature(signature));
                        }
                        LSPResult::Renames(workspace_edit) => {
                            self.events.borrow_mut().workspace.push(workspace_edit.into());
                        }
                        LSPResult::Tokens(tokens) => {
                            if self.line_builder.set_tokens(tokens) {
                                self.events.borrow_mut().overwrite("LSP tokens mapped!");
                            };
                        }
                        LSPResult::Declaration(declaration) => {
                            self.events.borrow_mut().workspace.push(declaration.into());
                        }
                        LSPResult::Definition(definition) => {
                            self.events.borrow_mut().workspace.push(definition.into());
                        }
                        LSPResult::None => (),
                    }
                }
            } else {
                unresolved_requests.push(request);
            }
        }
        self.requests = unresolved_requests;
        None
    }

    fn line_select(&mut self, at_line: usize, max_len: usize) -> Option<std::ops::Range<usize>> {
        let (from, to) = self.select?;
        if from.line > at_line || at_line > to.line {
            None
        } else if from.line < at_line && at_line < to.line {
            Some(0..max_len)
        } else if from.line == at_line && at_line == to.line {
            Some(from.char..to.char)
        } else if from.line == at_line {
            Some(from.char..max_len)
        } else {
            Some(0..to.char)
        }
    }

    pub fn list_item<'a>(&mut self, idx: usize, content: &'a str) -> ListItem<'a> {
        let spans = vec![Span::styled(
            get_line_num(idx, self.max_digits),
            Style { fg: Some(Color::Gray), ..Default::default() },
        )];
        self.line_builder.select_range = self.line_select(idx, content.len());
        ListItem::new(self.line_builder.build_line(idx, spans, content))
    }

    pub fn reload_theme(&mut self) {
        self.line_builder.theme = Theme::new();
        if let Some(client) = self.lsp_client.as_mut() {
            self.line_builder.map_styles(&client.capabilities.semantic_tokens_provider);
        }
    }
}

fn get_line_num(idx: usize, max_digits: usize) -> String {
    let mut as_str = (idx + 1).to_string();
    while as_str.len() < max_digits {
        as_str.insert(0, ' ')
    }
    as_str.push(' ');
    as_str
}
