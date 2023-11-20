mod line_builder;
mod modal;
mod theme;
use self::line_builder::LineBuilder;
use self::modal::{LSPModal, LSPModalResult, LSPResponseType, LSPResult};
pub use self::theme::{Theme, DEFAULT_THEME_FILE};
use crate::components::workspace::CursorPosition;
use crate::configs::EditorAction;
use crate::configs::FileType;
use crate::events::Events;
use crate::lsp::LSP;
use anyhow::anyhow;
use lsp_types::{PublishDiagnosticsParams, ServerCapabilities, TextDocumentContentChangeEvent, WorkspaceEdit};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::{widgets::ListItem, Frame};
use std::cell::RefCell;
use std::fmt::Debug;
use std::path::PathBuf;
use std::{path::Path, rc::Rc};
use tokio::sync::{Mutex, MutexGuard};

pub struct Lexer {
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub workspace_edit: Option<WorkspaceEdit>,
    pub events: Rc<RefCell<Events>>,
    pub lsp: Option<Rc<Mutex<LSP>>>,
    capabilities: ServerCapabilities,
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
            line_builder: (theme, file_type.into()).into(),
            select: None,
            modal: None,
            requests: Vec::new(),
            max_digits: 0,
            diagnostics: None,
            workspace_edit: None,
            events: Rc::clone(events),
            lsp: None,
            capabilities: ServerCapabilities::default(),
        }
    }

    pub fn context(
        &mut self,
        content: &[String],
        select: Option<(&CursorPosition, &CursorPosition)>,
        path: &Path,
    ) -> usize {
        self.get_diagnostics(path);
        self.get_lsp_responses();
        self.line_builder.reset();
        self.select = select.map(|(from, to)| (*from, *to));
        self.max_digits = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
        self.max_digits
    }

    pub async fn update_lsp(&mut self, path: &Path, changes: Option<(i32, Vec<TextDocumentContentChangeEvent>)>) {
        if let Some((version, content_changes)) = changes {
            self.line_builder.collect_changes(&content_changes);
            let mut error = None;
            let mut restart = None;
            if let Some(mut lsp) = self.try_expose_lsp() {
                match lsp.check_status().await {
                    Ok(None) => {
                        let _ = lsp.file_did_change(path, version, content_changes).await;
                    }
                    Ok(Some(err)) => {
                        error.replace(err);
                    }
                    Err(err) => {
                        error.replace(anyhow!("LSP crashed!"));
                        restart.replace(err.to_string());
                    }
                };
            }
            if let Some(err) = error {
                let mut events = self.events.borrow_mut();
                events.overwrite(err.to_string());
                events.message(restart.unwrap_or("LSP restarted!".to_string()));
            }
        }
        if self.line_builder.should_update() {
            self.line_builder.waiting = true;
            if self.get_tokens(path).await.is_some() {
                self.events.borrow_mut().message("Getting LSP syntax");
            };
        }
    }

    pub fn try_expose_lsp(&mut self) -> Option<MutexGuard<'_, LSP>> {
        let lsp_mutex = self.lsp.as_mut()?;
        match lsp_mutex.try_lock() {
            Ok(lsp) => Some(lsp),
            Err(err) => {
                self.events.borrow_mut().overwrite(format!("Failed to aquirre lsp: {err}"));
                None
            }
        }
    }

    pub fn render_modal_if_exist(&mut self, frame: &mut Frame, x: u16, y: u16) {
        if let Some(modal) = &mut self.modal {
            modal.render_at(frame, x, y);
        }
    }

    pub async fn map_modal_if_exists(&mut self, key: &EditorAction, path: &Path) -> bool {
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
                    self.get_renames(path, &c, new_name).await;
                    self.modal.take();
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    pub async fn set_lsp(&mut self, lsp: Rc<Mutex<LSP>>, on_file: &PathBuf) {
        self.events.borrow_mut().message("Mapping LSP ...");
        {
            let mut guard = lsp.lock().await;
            if guard.file_did_open(on_file).await.is_err() {
                return;
            }
            self.capabilities = guard.initialized.capabilities.clone();
        }
        self.line_builder.map_styles(&self.capabilities.semantic_tokens_provider);
        self.events.borrow_mut().overwrite("LSP mapped!");
        self.lsp.replace(lsp);
    }

    fn get_diagnostics(&mut self, path: &Path) -> Option<()> {
        let params = self.try_expose_lsp()?.get_diagnostics(path)?;
        self.line_builder.set_diganostics(params);
        Some(())
    }

    pub fn start_renames(&mut self, c: &CursorPosition, title: &str) {
        if let Some(lsp) = self.try_expose_lsp() {
            if lsp.initialized.capabilities.rename_provider.is_none() {
                return;
            }
        }
        self.modal.replace(LSPModal::renames_at(*c, title));
    }

    pub async fn get_renames(&mut self, path: &Path, c: &CursorPosition, new_name: String) -> Option<()> {
        self.capabilities.rename_provider.as_ref()?;
        let id = self.try_expose_lsp()?.renames(path, c, new_name).await?;
        self.requests.push(LSPResponseType::Renames(id));
        Some(())
    }

    pub async fn get_autocomplete(&mut self, path: &Path, c: &CursorPosition, line: &str) -> Option<()> {
        if matches!(self.modal, Some(LSPModal::AutoComplete(..))) || !self.line_builder.lang.completelable(line, c.char)
        {
            return None;
        }
        let id = self.try_expose_lsp()?.completion(path, c).await?;
        self.requests.push(LSPResponseType::Completion(id, line.to_owned(), c.char));
        Some(())
    }

    pub async fn get_hover(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.capabilities.hover_provider.as_ref()?;
        let id = self.try_expose_lsp()?.hover(path, c).await?;
        self.requests.push(LSPResponseType::Hover(id));
        Some(())
    }

    pub async fn go_to_declaration(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.capabilities.declaration_provider.as_ref()?;
        let id = self.try_expose_lsp()?.declaration(path, c).await?;
        self.requests.push(LSPResponseType::Declaration(id));
        Some(())
    }

    pub async fn go_to_definition(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.capabilities.definition_provider.as_ref()?;
        let id = self.try_expose_lsp()?.definition(path, c).await?;
        self.requests.push(LSPResponseType::Definition(id));
        Some(())
    }

    pub async fn get_signitures(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        self.capabilities.signature_help_provider.as_ref()?;
        let id = self.try_expose_lsp()?.signiture_help(path, c).await?;
        self.requests.push(LSPResponseType::SignatureHelp(id));
        Some(())
    }

    pub async fn get_tokens(&mut self, path: &Path) -> Option<()> {
        self.capabilities.semantic_tokens_provider.as_ref()?;
        let id = self.try_expose_lsp()?.semantics(path).await?;
        self.requests.push(LSPResponseType::TokensFull(id));
        Some(())
    }

    fn get_lsp_responses(&mut self) -> Option<()> {
        if self.requests.is_empty() {
            return None;
        }
        let lsp = self.lsp.as_mut()?.try_lock().ok()?;
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

    pub fn new_theme(&mut self, theme: Theme) {
        self.line_builder.theme = theme;
        if let Some(lsp_rc) = self.lsp.as_mut() {
            if let Ok(lsp) = lsp_rc.try_lock() {
                self.line_builder.map_styles(&lsp.initialized.capabilities.semantic_tokens_provider);
            }
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
