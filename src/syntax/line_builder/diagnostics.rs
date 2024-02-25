use super::Lang;
use crate::global_state::WorkspaceEvent;
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

impl DiagnosticInfo {}

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

impl Action {
    pub fn to_string(&self) -> String {
        match self {
            Self::Import(text) => format!("import {text}"),
        }
    }
}

pub struct DiagnosticData {
    pub start: usize,
    pub end: Option<usize>,
    pub span: Span<'static>,
    pub info: Option<Vec<DiagnosticRelatedInformation>>,
}

impl DiagnosticData {
    fn new(
        range: lsp_types::Range,
        message: String,
        color: Style,
        info: Option<Vec<DiagnosticRelatedInformation>>,
    ) -> Self {
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            span: Span::styled(format!("    {}", message), color),
            info,
        }
    }
}

pub struct DiagnosticLine {
    pub data: Vec<DiagnosticData>,
}

impl DiagnosticLine {
    pub fn check_ranges(&self, idx: usize) -> Option<Color> {
        for data in self.data.iter() {
            match data.end {
                Some(end_idx) if (data.start..end_idx).contains(&idx) => return data.span.style.fg,
                None if idx >= data.start => return data.span.style.fg,
                _ => {}
            }
        }
        None
    }

    pub fn collect_info(&self, lang: &Lang) -> DiagnosticInfo {
        let mut info = DiagnosticInfo::default();
        let mut buffer = Vec::new();
        for diagnostic in self.data.iter() {
            info.messages.push(diagnostic.span.clone());
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
        self.data.retain(|d| d.span.style.fg == Some(ERR_COLOR));
    }

    pub fn set_diagnostic_style(&self, idx: usize, style: &mut Style) {
        if let Some(color) = self.check_ranges(idx) {
            style.add_modifier = style.add_modifier.union(Modifier::UNDERLINED);
            style.underline_color.replace(color);
        }
    }

    pub fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_STYLE, d.related_information));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].span.style.fg {
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
    builder.diagnostics.extend(diagnostics.into_iter().map(|(idx, mut line)| {
        line.drop_non_errs();
        (idx, line)
    }));
}

pub fn diagnostics_full(builder: &mut LineBuilder, diagnostics: Vec<(usize, DiagnosticLine)>) {
    builder.diagnostics.extend(diagnostics);
}
