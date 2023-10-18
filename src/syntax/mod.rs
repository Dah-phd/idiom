mod langs;
mod lsp_tokens;
mod modal;
mod rust;
mod theme;
use self::modal::{AutoComplete, Info, LSPResponseType, LSPResult, Modal};
pub use self::theme::{Theme, DEFAULT_THEME_FILE};
use crate::components::editor::CursorPosition;
use crate::configs::EditorAction;
use crate::configs::FileType;
use crate::lsp::LSP;
use langs::Lang;
use lsp_types::{DiagnosticSeverity, PublishDiagnosticsParams, WorkspaceEdit};
use ratatui::{
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};
use std::fmt::Debug;
use std::{io::Stdout, path::Path, rc::Rc};
use tokio::sync::{Mutex, MutexGuard};
pub const COLORS: [Color; 3] = [Color::LightMagenta, Color::Yellow, Color::Blue];

pub struct Lexer {
    pub select: Option<(CursorPosition, CursorPosition)>,
    pub theme: Theme,
    pub diagnostics: Option<PublishDiagnosticsParams>,
    pub lsp: Option<Rc<Mutex<LSP>>>,
    pub workspace_edit: Option<WorkspaceEdit>,
    modal: Option<Box<dyn Modal>>,
    requests: Vec<LSPResponseType>,
    line_processor: fn(&mut Lexer, content: &str, spans: &mut Vec<Span>),
    lang: Lang,
    select_at_line: Option<(usize, usize)>,
    curly: Vec<Color>,
    brackets: Vec<Color>,
    square: Vec<Color>,
    token_start: usize,
    last_token: String,
    max_digits: usize,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lexer")
    }
}

impl Lexer {
    pub fn from_type(file_type: &FileType, theme: Theme) -> Self {
        Self {
            select: None,
            theme,
            diagnostics: None,
            lsp: None,
            modal: None,
            requests: Vec::new(),
            line_processor: derive_line_processor(file_type),
            lang: file_type.into(),
            select_at_line: None,
            workspace_edit: None,
            curly: vec![],
            brackets: vec![],
            square: vec![],
            token_start: 0,
            last_token: String::default(),
            max_digits: 0,
        }
    }

    pub fn context(
        &mut self,
        content: &[String],
        c: &CursorPosition,
        select: Option<(&CursorPosition, &CursorPosition)>,
        path: &Path,
    ) -> usize {
        self.get_diagnostics(path);
        self.get_lsp_responses(c);
        self.curly.clear();
        self.brackets.clear();
        self.square.clear();
        self.select = select.map(|(from, to)| (*from, *to));
        self.max_digits = if content.is_empty() { 0 } else { (content.len().ilog10() + 1) as usize };
        self.max_digits
    }

    pub fn expose_lsp(&mut self) -> Option<MutexGuard<'_, LSP>> {
        let lsp_mutex = self.lsp.as_mut()?;
        lsp_mutex.try_lock().ok()
    }

    pub fn render_modal_if_exist(&mut self, frame: &mut Frame<CrosstermBackend<&Stdout>>, x: u16, y: u16) {
        if let Some(modal) = self.modal.as_mut() {
            modal.render_at(frame, x, y);
        }
    }

    pub fn map_modal_if_exists(&mut self, key: &EditorAction) {
        if let Some(modal) = self.modal.as_mut() {
            if modal.map_and_finish(key) {
                self.modal = None;
            }
        }
    }

    fn get_diagnostics(&mut self, path: &Path) -> Option<()> {
        let diagnostics = self.expose_lsp()?.get_diagnostics(path);
        if diagnostics.is_some() {
            self.diagnostics = diagnostics;
        }
        Some(())
    }

    pub async fn get_renames(&mut self, path: &Path, c: &CursorPosition, new_name: String) -> Option<()> {
        let id = self.expose_lsp()?.renames(path, c, new_name).await?;
        self.requests.push(LSPResponseType::Renames(id));
        Some(())
    }

    pub async fn get_autocomplete(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        if self.modal.is_some() {
            return None;
        }
        let id = self.expose_lsp()?.completion(path, c).await?;
        self.requests.push(LSPResponseType::Completion(id));
        Some(())
    }

    pub async fn get_hover(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        if self.modal.is_some() {
            return None;
        }
        let id = self.expose_lsp()?.hover(path, c).await?;
        self.requests.push(LSPResponseType::Hover(id));
        Some(())
    }

    pub async fn get_signitures(&mut self, path: &Path, c: &CursorPosition) -> Option<()> {
        let id = self.expose_lsp()?.signiture_help(path, c).await?;
        self.requests.push(LSPResponseType::SignatureHelp(id));
        Some(())
    }

    fn get_lsp_responses(&mut self, c: &CursorPosition) -> Option<()> {
        if self.requests.is_empty() {
            return None;
        }
        let lsp = self.lsp.as_mut()?.try_lock().ok()?;
        let request = self.requests.remove(0);
        if let Some(response) = lsp.get(request.id()) {
            if let Some(value) = response.result {
                match request.parse(value) {
                    LSPResult::Completion(completion) => {
                        self.modal = Some(Box::new(AutoComplete::new(c, completion)));
                    }
                    LSPResult::Hover(hover) => {
                        self.modal = Some(Box::new(Info::from_hover(hover)));
                    }
                    LSPResult::SignatureHelp(signature) => {
                        self.modal = Some(Box::new(Info::from_signature(signature)));
                    }
                    LSPResult::Renames(workspace_edit) => self.workspace_edit = Some(workspace_edit),
                    LSPResult::None => (),
                }
            }
        } else {
            self.requests.push(request);
        }
        None
    }

    fn set_select_char_range(&mut self, at_line: usize, max_len: usize) {
        if let Some((from, to)) = self.select {
            if from.line > at_line || at_line > to.line {
                self.select_at_line = None;
            } else if from.line < at_line && at_line < to.line {
                self.select_at_line = Some((0, max_len));
            } else if from.line == at_line && at_line == to.line {
                self.select_at_line = Some((from.char, to.char));
            } else if from.line == at_line {
                self.select_at_line = Some((from.char, max_len));
            } else if to.line == at_line {
                self.select_at_line = Some((0, to.char))
            }
        } else {
            self.select_at_line = None
        }
    }

    fn handled_key_word(&mut self, token_end: usize, spans: &mut Vec<Span>) -> bool {
        if self.lang.key_words.contains(&self.last_token.trim()) {
            self.drain_with_select(token_end, self.theme.key_words, spans);
            return true;
        }
        if self.lang.frow_control.contains(&self.last_token.trim()) {
            self.drain_with_select(token_end, self.theme.flow_control, spans);
            return true;
        }
        false
    }

    fn handled_object(&mut self, token_end: usize, spans: &mut Vec<Span>) -> bool {
        if let Some(ch) = self.last_token.trim().chars().next() {
            if ch.is_uppercase() {
                self.drain_with_select(token_end, self.theme.class_or_struct, spans);
                return true;
            }
        }
        false
    }

    pub fn syntax_spans<'a>(&mut self, idx: usize, content: &'a str) -> ListItem<'a> {
        let mut spans = vec![Span::styled(
            get_line_num(idx, self.max_digits),
            Style::default().fg(Color::Gray),
        )];
        self.set_select_char_range(idx, content.len());
        self.token_start = 0;
        if self.select_at_line.is_some() && content.is_empty() {
            spans.push(Span {
                content: " ".into(),
                style: Style { bg: Some(self.theme.selected), ..Default::default() },
            })
        } else {
            (self.line_processor)(self, content, &mut spans);
            if !self.last_token.is_empty() {
                self.drain_buf(content.len().checked_sub(1).unwrap_or_default(), &mut spans);
            }
            if let Some(diagnostics) = &self.diagnostics {
                for diagnostic in diagnostics.diagnostics.iter() {
                    if idx == diagnostic.range.start.line as usize {
                        match diagnostic.severity {
                            Some(severity) => match severity {
                                DiagnosticSeverity::ERROR => spans.push(Span::styled(
                                    format!("    {}", diagnostic.message),
                                    Style::default().fg(Color::Red),
                                )),
                                DiagnosticSeverity::WARNING => spans.push(Span::styled(
                                    format!("    {}", diagnostic.message),
                                    Style::default().fg(Color::LightYellow),
                                )),
                                _ => spans.push(Span::styled(
                                    format!("    {}", diagnostic.message),
                                    Style::default().fg(Color::Gray),
                                )),
                            },
                            None => spans.push(Span::styled(
                                format!("    {}", diagnostic.message),
                                Style::default().fg(Color::Gray),
                            )),
                        }
                    }
                }
            }
        }
        ListItem::new(Line::from(spans))
    }

    fn white_char(&mut self, idx: usize, ch: char, spans: &mut Vec<Span>) {
        if matches!(self.select_at_line, Some((from, to)) if from <= idx && idx < to) {
            spans.push(Span::styled(
                String::from(ch),
                Style { bg: Some(self.theme.selected), fg: Some(Color::White), ..Default::default() },
            ));
        } else {
            spans.push(Span::styled(String::from(ch), Style { fg: Some(Color::White), ..Default::default() }))
        }
        self.token_start += 1;
    }

    fn drain_buf_object(&mut self, token_end: usize, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) {
            self.drain_with_select(token_end, self.theme.class_or_struct, spans)
        }
    }

    fn drain_buf_colored(&mut self, token_end: usize, color: Color, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) && !self.handled_object(token_end, spans) {
            self.drain_with_select(token_end, color, spans)
        }
    }

    fn drain_buf(&mut self, token_end: usize, spans: &mut Vec<Span>) {
        if !self.handled_key_word(token_end, spans) && !self.handled_object(token_end, spans) {
            self.drain_with_select(token_end, self.theme.default, spans)
        }
    }

    #[allow(clippy::collapsible_else_if)]
    fn drain_with_select(&mut self, token_end: usize, color: Color, spans: &mut Vec<Span>) {
        let style = Style { fg: Some(color), ..Default::default() };
        if let Some((select_start, select_end)) = self.select_at_line {
            if select_start <= self.token_start && token_end < select_end {
                spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style.bg(self.theme.selected)));
            } else if select_end <= self.token_start || token_end <= select_start {
                spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
            } else {
                if select_start <= self.token_start {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_end - self.token_start),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else if self.token_start <= select_start && select_end <= token_end {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_start - self.token_start),
                        style,
                    ));
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_end - select_start),
                        style.bg(self.theme.selected),
                    ));
                    spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
                } else {
                    spans.push(Span::styled(
                        drain_token_checked(&mut self.last_token, select_start - self.token_start),
                        style,
                    ));
                    spans.push(Span::styled(
                        self.last_token.drain(..).collect::<String>(),
                        style.bg(self.theme.selected),
                    ));
                };
            }
        } else {
            spans.push(Span::styled(self.last_token.drain(..).collect::<String>(), style));
        }
        self.token_start = token_end;
    }

    fn default_color() -> Color {
        COLORS[COLORS.len() - 1]
    }

    fn len_to_color(len: usize) -> Color {
        COLORS[len % COLORS.len()]
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

fn drain_token_checked(token: &mut String, last_idx: usize) -> String {
    if last_idx >= token.len() {
        token.drain(..).collect()
    } else {
        token.drain(..last_idx).collect()
    }
}

fn derive_line_processor(file_type: &FileType) -> fn(&mut Lexer, content: &str, spans: &mut Vec<Span>) {
    match file_type {
        FileType::Rust => rust::rust_processor,
        _ => rust::rust_processor,
    }
}
