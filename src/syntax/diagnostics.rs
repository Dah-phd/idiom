use crate::syntax::{Lang, Token};
use crate::{global_state::WorkspaceEvent, workspace::line::Line};
use crossterm::style::{Attribute, Color};
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity};

const ELS_COLOR: Color = Color::DarkGrey;
const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::Yellow;
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
    pub inline_span: (String, Color),
    pub message: (String, Color),
    pub info: Option<Vec<DiagnosticRelatedInformation>>,
}

impl DiagnosticData {
    fn new(
        range: lsp_types::Range,
        message: String,
        color: Color,
        info: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Self {
        let first_line_fmt = message.lines().next().map(|s| format!("    {s}")).unwrap_or_default();
        let inline_span = (first_line_fmt, color);
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            inline_span,
            message: (message, color),
            info,
        }
    }

    pub fn check_and_update(&self, token: &mut Token) {
        match self.end {
            Some(end) if self.start <= token.from && token.to <= end => {
                token.color.underline_color = Some(self.inline_span.1);
                token.color.attributes.set(Attribute::Undercurled);
            }
            None if self.start <= token.from => {
                token.color.underline_color = Some(self.inline_span.1);
                token.color.attributes.set(Attribute::Undercurled);
            }
            _ => {}
        }
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
            info.messages.push(diagnostic.message.clone());
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

    pub fn drop_non_errs(&mut self) {
        self.data.retain(|d| d.inline_span.1 == ERR_COLOR);
    }

    pub fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_COLOR, d.related_information));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].inline_span.1 {
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

pub fn set_diganostics(content: &mut Vec<impl Line>, diagnostics: Vec<(usize, DiagnosticLine)>) {
    for line in content.iter_mut() {
        line.drop_diagnostics();
    }
    for (idx, diagnostics) in diagnostics {
        content[idx].set_diagnostics(diagnostics);
    }
}

pub fn set_diganostic_errors(content: &mut Vec<impl Line>, diagnostics: Vec<(usize, DiagnosticLine)>) {
    for line in content.iter_mut() {
        line.drop_diagnostics();
    }
    for (idx, mut diagnostics) in diagnostics {
        diagnostics.drop_non_errs();
        content[idx].set_diagnostics(diagnostics);
    }
}
