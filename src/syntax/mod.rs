mod langs;
mod line_builder;
mod modal;
mod theme;
use self::line_builder::LineBuilder;
use self::modal::{should_complete, LSPModal, LSPModalResult, LSPResponseType, LSPResult};
pub use self::theme::{Theme, DEFAULT_THEME_FILE};
use crate::components::workspace::CursorPosition;
use crate::configs::EditorAction;
use crate::configs::FileType;
use crate::events::Events;
use crate::lsp::LSP;
use anyhow::anyhow;
use lsp_types::{PublishDiagnosticsParams, TextDocumentContentChangeEvent, WorkspaceEdit};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::{prelude::CrosstermBackend, widgets::ListItem, Frame};
use std::cell::RefCell;
use std::fmt::Debug;
use std::path::PathBuf;
use std::{io::Stdout, path::Path, rc::Rc};
use tokio::sync::{Mutex, MutexGuard};

pub struct Lexer {
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp: Option<Rc<Mutex<LSP>>>,
    pub workspace_edit: Option<WorkspaceEdit>,
    pub events: Rc<RefCell<Events>>,
    line_builder: LineBuilder,
    ft: FileType,
    select: Option<(CursorPosition, CursorPosition)>,
    modal: LSPModal,
    requests: Vec<LSPResponseType>,
    max_digits: usize,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("LEXER: {:?}", self.ft).as_str())
    }
}

impl Lexer {
    pub fn from_type(file_type: &FileType, theme: Theme, events: &Rc<RefCell<Events>>) -> Self {
        Self {
            line_builder: (theme, file_type.into()).into(),
            ft: *file_type,
            select: None,
            modal: LSPModal::default(),
            requests: Vec::new(),
            max_digits: 0,
            diagnostics: None,
            lsp: None,
            workspace_edit: None,
            events: Rc::clone(events),
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
        lsp_mutex.try_lock().ok()
    }

    pub fn render_modal_if_exist(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        self.modal.render_at(frame, x, y);
    }

    pub fn map_modal_if_exists(&mut self, key: &EditorAction) -> bool {
        match self.modal.map_and_finish(key) {
            LSPModalResult::Done => self.modal.clear(),
            LSPModalResult::Teken => return true,
            LSPModalResult::TakenDone => {
                self.modal.clear();
                return true;
            }
            LSPModalResult::Workspace(event) => {
                self.events.borrow_mut().workspace.push(event);
                return true;
            }
            _ => (),
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
            self.line_builder.map_styles(&self.ft, &guard.initialized.capabilities.semantic_tokens_provider);
        }
        self.events.borrow_mut().overwrite("LSP mapped!");
        self.lsp.replace(lsp);
    }

    fn get_diagnostics(&mut self, path: &Path) -> Option<()> {
        let params = self.try_expose_lsp()?.get_diagnostics(path)?;
        self.line_builder.set_diganostics(params);
        Some(())
    }

    pub async fn get_renames(&mut self, path: &Path, c: &CursorPosition, new_name: String) -> Option<()> {
        let id = self.try_expose_lsp()?.renames(path, c, new_name).await?;
        self.requests.push(LSPResponseType::Renames(id));
        Some(())
    }

    pub async fn get_autocomplete(&mut self, path: &Path, c: &CursorPosition, line: &str) -> Option<()> {
        if matches!(self.modal, LSPModal::AutoComplete(..)) || !should_complete(line, c.char) {
            return None;
        }
        let id = self.try_expose_lsp()?.completion(path, c).await?;
        self.requests.push(LSPResponseType::Completion(id, line.to_owned()));
        Some(())
    }

    pub async fn get_hover(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        let id = self.try_expose_lsp()?.hover(path, c).await?;
        self.requests.push(LSPResponseType::Hover(id));
        Some(())
    }

    pub async fn get_signitures(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        let id = self.try_expose_lsp()?.signiture_help(path, c).await?;
        self.requests.push(LSPResponseType::SignatureHelp(id));
        Some(())
    }

    pub async fn get_tokens(&mut self, path: &Path) -> Option<()> {
        let id = self.try_expose_lsp()?.semantics(path).await?;
        self.requests.push(LSPResponseType::TokensFull(id));
        Some(())
    }

    fn get_lsp_responses(&mut self) -> Option<()> {
        if self.requests.is_empty() {
            return None;
        }
        let lsp = self.lsp.as_mut()?.try_lock().ok()?;
        let request = self.requests.remove(0);
        if let Some(response) = lsp.get(request.id()) {
            if let Some(value) = response.result {
                match request.parse(value) {
                    LSPResult::Completion(completions, line) => self.modal.auto_complete(completions, line),
                    LSPResult::Hover(hover) => self.modal.hover(hover),
                    LSPResult::SignatureHelp(signature) => self.modal.signature(signature),
                    LSPResult::Renames(workspace_edit) => self.workspace_edit = Some(workspace_edit),
                    LSPResult::Tokens(tokens) => {
                        if self.line_builder.set_tokens(tokens) {
                            self.events.borrow_mut().overwrite("LSP tokens mapped!");
                        };
                    }
                    LSPResult::None => (),
                }
            }
        } else {
            self.requests.push(request);
        }
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
                self.line_builder.map_styles(&self.ft, &lsp.initialized.capabilities.semantic_tokens_provider);
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
