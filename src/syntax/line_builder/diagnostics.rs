use crate::global_state::WorkspaceEvent;
use crate::syntax::line_builder::tokens::Token;
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity};

use super::LineBuilder;

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

const ELS_COLOR: Color = Color::Gray;
const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::LightYellow;
const ERR_STYLE: Style = Style::new().fg(ERR_COLOR);
const WAR_STYLE: Style = Style::new().fg(WAR_COLOR);
const ELS_STYLE: Style = Style::new().fg(ELS_COLOR);

#[derive(Default)]
pub struct DiagnosticInfo {
    pub messages: Vec<Span<'static>>,
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
    pub inline_span: Span<'static>,
    pub message: Span<'static>,
    pub info: Option<Vec<DiagnosticRelatedInformation>>,
}

impl DiagnosticData {
    fn new(
        range: lsp_types::Range,
        message: String,
        color: Style,
        info: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Self {
        let first_line_fmt = message.lines().next().map(|s| format!("    {s}")).unwrap_or_default();
        let inline_span = Span::styled(first_line_fmt, color);
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            inline_span,
            message: Span::styled(message, color),
            info,
        }
    }

    pub fn check_token(&self, token: &mut Token) {
        match self.end {
            Some(end) => {
                if self.start <= token.from && token.to <= end {
                    token.color.underline_color = self.inline_span.style.fg;
                    token.color.add_modifier = Modifier::UNDERLINED;
                }
            }
            None if self.start <= token.from => {
                token.color.underline_color = self.inline_span.style.fg;
                token.color.add_modifier = Modifier::UNDERLINED;
            }
            _ => {}
        }
    }
}

pub struct DiagnosticLine {
    pub data: Vec<DiagnosticData>,
}

impl DiagnosticLine {
    pub fn drop_non_errs(&mut self) {
        self.data.retain(|d| d.inline_span.style.fg == Some(ERR_COLOR));
    }

    pub fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_STYLE, d.related_information));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].inline_span.style.fg {
                Some(ELS_COLOR) => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_STYLE, d.related_information));
                }
                _ => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_STYLE, d.related_information));
                }
            },
            _ => {
                self.data.push(DiagnosticData::new(d.range, d.message, ELS_STYLE, d.related_information));
            }
        }
    }
}

impl From<Diagnostic> for DiagnosticLine {
    fn from(diagnostic: Diagnostic) -> Self {
        let color = match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => ERR_STYLE,
            Some(DiagnosticSeverity::WARNING) => WAR_STYLE,
            _ => ELS_STYLE,
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

pub fn diagnostics_error(builder: &mut LineBuilder, diagnostics: Vec<(usize, DiagnosticLine)>) {
    builder.tokens.set_diagnositc_errors(diagnostics);
}

pub fn diagnostics_full(builder: &mut LineBuilder, diagnostics: Vec<(usize, DiagnosticLine)>) {
    builder.tokens.set_diagnostics(diagnostics);
    builder.diagnostic_processor = diagnostics_error;
}
