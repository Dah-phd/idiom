use crate::global_state::WorkspaceEvent;
use crate::render::backend::{color, BackendProtocol, Color, Style};
use crate::render::UTF8Safe;
use crate::syntax::Lang;
use crate::workspace::line::CodeLine;
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity};

const ELS_COLOR: Color = color::dark_grey();
const ERR_COLOR: Color = color::red();
const WAR_COLOR: Color = color::yellow();
// const ERR_STYLE: Style = Style::new().fg(ERR_COLOR);
// const WAR_STYLE: Style = Style::new().fg(WAR_COLOR);
// const ELS_STYLE: Style = Style::new().fg(ELS_COLOR);

#[derive(Default)]
pub struct DiagnosticInfo {
    pub messages: Vec<(String, Color)>,
    pub actions: Option<Vec<Action>>,
}

#[derive(Clone)]
pub enum Action {
    Import(String),
}

impl From<Action> for WorkspaceEvent {
    fn from(value: Action) -> Self {
        match value {
            Action::Import(text) => WorkspaceEvent::InsertText(text),
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(text) => write!(f, "import {text}"),
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
    pub fn text_style(&self) -> Style {
        Style::fg(self.color)
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
    pub fn inline_render(&self, max_width: usize, backend: &mut impl BackendProtocol) {
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

pub fn set_diganostics(content: &mut [CodeLine], diagnostics: Vec<(usize, DiagnosticLine)>) {
    for line in content.iter_mut() {
        line.drop_diagnostics();
    }
    for (idx, diagnostics) in diagnostics {
        if let Some(line) = content.get_mut(idx) {
            line.set_diagnostics(diagnostics);
        };
    }
}
