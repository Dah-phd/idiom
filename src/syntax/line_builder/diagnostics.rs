use lsp_types::{Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams};

use super::LineBuilder;

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::collections::hash_map::Entry;

const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::LightYellow;
const ELS_COLOR: Color = Color::Gray;

#[derive(Debug)]
pub struct DiagnosticData {
    pub start: usize,
    pub end: Option<usize>,
    pub span: Span<'static>,
}

impl DiagnosticData {
    fn new(range: lsp_types::Range, message: String, color: Color) -> Self {
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            span: Span::styled(format!("    {}", message), Style { fg: Some(color), ..Default::default() }),
        }
    }
}

#[derive(Debug)]
pub struct DiagnosticLines {
    pub data: Vec<DiagnosticData>,
}

impl DiagnosticLines {
    pub fn check_ranges(&self, idx: &usize) -> Option<Color> {
        for data in self.data.iter() {
            match data.end {
                Some(end_idx) if (data.start..end_idx).contains(idx) => return data.span.style.fg,
                None if idx >= &data.start => return data.span.style.fg,
                _ => {}
            }
        }
        None
    }

    pub fn drop_non_errs(&mut self) {
        self.data.retain(|d| d.span.style.fg == Some(ERR_COLOR));
    }

    fn append(&mut self, d: Diagnostic) {
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_COLOR));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].span.style.fg {
                Some(ELS_COLOR) => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_COLOR));
                }
                _ => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_COLOR));
                }
            },
            _ => {
                self.data.push(DiagnosticData::new(d.range, d.message, ELS_COLOR));
            }
        }
    }
}

impl From<Diagnostic> for DiagnosticLines {
    fn from(diagnostic: Diagnostic) -> Self {
        let color = match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => ERR_COLOR,
            Some(DiagnosticSeverity::WARNING) => WAR_COLOR,
            _ => ELS_COLOR,
        };
        Self { data: vec![DiagnosticData::new(diagnostic.range, diagnostic.message, color)] }
    }
}

pub fn diagnostics_error(builder: &mut LineBuilder, params: PublishDiagnosticsParams) {
    for d in params.diagnostics {
        if !matches!(d.severity, Some(DiagnosticSeverity::ERROR)) {
            continue;
        }
        match builder.diagnostics.entry(d.range.start.line as usize) {
            Entry::Occupied(mut e) => {
                e.get_mut().append(d);
            }
            Entry::Vacant(e) => {
                e.insert(d.into());
            }
        }
    }
}

pub fn diagnostics_full(builder: &mut LineBuilder, params: PublishDiagnosticsParams) {
    for diagnostic in params.diagnostics {
        match builder.diagnostics.entry(diagnostic.range.start.line as usize) {
            Entry::Occupied(mut e) => {
                e.get_mut().append(diagnostic);
            }
            Entry::Vacant(e) => {
                e.insert(diagnostic.into());
            }
        };
    }
}
