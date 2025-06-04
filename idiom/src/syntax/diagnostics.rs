use crate::global_state::IdiomEvent;
use crate::syntax::Lang;
use crate::workspace::line::EditorLine;
use crossterm::style::{Color, ContentStyle};
use idiom_ui::backend::{Backend, StyleExt};
use idiom_ui::UTF8Safe;
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity};

const ELS_COLOR: Color = Color::DarkGrey;
const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::Yellow;

#[derive(Default)]
pub struct DiagnosticInfo {
    pub messages: Vec<(String, Color)>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Clone)]
pub enum Action {
    Import(String),
}

impl From<Action> for IdiomEvent {
    fn from(value: Action) -> Self {
        match value {
            Action::Import(text) => IdiomEvent::InsertText(text),
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(text) => match text.strip_suffix('\n') {
                Some(stripped_text) => write!(f, "import {stripped_text}"),
                None => write!(f, "import {text}"),
            },
        }
    }
}

pub struct DiagnosticData {
    pub start: usize,
    pub end: Option<usize>,
    pub color: Color,
    pub inline_text: String,
    pub message: String,
    pub info: Option<Vec<DiagnosticRelatedInformation>>,
}

impl DiagnosticData {
    fn new(
        range: lsp_types::Range,
        message: String,
        color: Color,
        info: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Self {
        let inline_text = message.lines().next().map(|s| format!("    {s}")).unwrap_or_default();
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            color,
            inline_text,
            message,
            info,
        }
    }

    #[inline]
    pub fn truncated_inline(&self, len: usize) -> &str {
        unsafe { self.inline_text.as_str().get_unchecked(..std::cmp::min(self.inline_text.len(), len)) }
    }

    #[inline]
    pub fn text_style(&self) -> ContentStyle {
        ContentStyle::fg(self.color)
    }
}

pub struct DiagnosticLine {
    pub data: Vec<DiagnosticData>,
}

impl DiagnosticLine {
    pub fn collect_info(&self, lang: &Lang) -> DiagnosticInfo {
        let mut info = DiagnosticInfo::default();
        let mut buffer = Vec::new();
        for diagnostic in self.data.iter() {
            info.messages.push((diagnostic.message.clone(), diagnostic.color));
            if let Some(actions) = lang.derive_diagnostic_actions(diagnostic.info.as_ref()) {
                for action in actions {
                    buffer.push(action.clone());
                }
            }
        }
        if !buffer.is_empty() {
            info.actions.replace(buffer);
        }
        info
    }

    /// Prints truncated text based on info from diagnostics
    #[inline(always)]
    pub fn inline_render(&self, max_width: usize, backend: &mut impl Backend) {
        if max_width < 5 {
            return;
        }
        if let Some(first_diagnostic) = self.data.first() {
            let style = first_diagnostic.text_style();
            let text = first_diagnostic.inline_text.truncate_width(max_width - 1).1;
            backend.print_styled(text, style);
        }
    }

    pub fn drop_non_errs(&mut self) {
        self.data.retain(|d| d.color == ERR_COLOR);
    }

    pub fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_COLOR, d.related_information));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].color {
                ELS_COLOR => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_COLOR, d.related_information));
                }
                _ => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_COLOR, d.related_information));
                }
            },
            _ => {
                self.data.push(DiagnosticData::new(d.range, d.message, ELS_COLOR, d.related_information));
            }
        }
    }
}

impl From<Diagnostic> for DiagnosticLine {
    fn from(diagnostic: Diagnostic) -> Self {
        let color = match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => ERR_COLOR,
            Some(DiagnosticSeverity::WARNING) => WAR_COLOR,
            _ => ELS_COLOR,
        };
        Self {
            data: vec![DiagnosticData::new(
                diagnostic.range,
                diagnostic.message,
                color,
                diagnostic.related_information,
            )],
        }
    }
}

pub fn set_diganostics(content: &mut [EditorLine], diagnostics: Vec<(usize, DiagnosticLine)>) {
    for line in content.iter_mut() {
        line.drop_diagnostics();
    }
    for (idx, diagnostics) in diagnostics {
        if let Some(line) = content.get_mut(idx) {
            line.set_diagnostics(diagnostics);
        };
    }
}
