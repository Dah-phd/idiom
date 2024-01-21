use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, PublishDiagnosticsParams};

use super::LineBuilder;

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::collections::hash_map::Entry;

const ELS_COLOR: Color = Color::Gray;
const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::LightYellow;
const ERR_STYLE: Style = Style::new().fg(ERR_COLOR);
const WAR_STYLE: Style = Style::new().fg(WAR_COLOR);
const ELS_STYLE: Style = Style::new().fg(ELS_COLOR);

#[derive(Debug)]
pub struct DiagnosticData {
    pub start: usize,
    pub end: Option<usize>,
    pub span: Span<'static>,
    pub actions: Option<Vec<String>>,
}

impl DiagnosticData {
    fn new(range: lsp_types::Range, message: String, color: Style, actions: Option<Vec<String>>) -> Self {
        let span = match actions {
            None => Span::styled(format!("    {}", message), color),
            Some(..) => Span::styled(format!("    💡 {}", message), color),
        };
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            span,
            actions,
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

    pub fn collect_actions(&self) -> Option<Vec<String>> {
        let mut buffer = Vec::new();
        for diagnostic in self.data.iter() {
            if let Some(actions) = diagnostic.actions.as_ref() {
                for action in actions {
                    buffer.push(action.to_owned());
                }
            }
        }
        if buffer.is_empty() {
            return None;
        }
        Some(buffer)
    }

    pub fn drop_non_errs(&mut self) {
        self.data.retain(|d| d.span.style.fg == Some(ERR_COLOR));
    }

    fn append(&mut self, d: Diagnostic) {
        let actions = process_related_info(d.related_information);
        match d.severity {
            Some(DiagnosticSeverity::ERROR) => {
                self.data.insert(0, DiagnosticData::new(d.range, d.message, ERR_STYLE, actions));
            }
            Some(DiagnosticSeverity::WARNING) => match self.data[0].span.style.fg {
                Some(ELS_COLOR) => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_STYLE, actions));
                }
                _ => {
                    self.data.insert(0, DiagnosticData::new(d.range, d.message, WAR_STYLE, actions));
                }
            },
            _ => {
                self.data.push(DiagnosticData::new(d.range, d.message, ELS_STYLE, actions));
            }
        }
    }
}

impl From<Diagnostic> for DiagnosticLines {
    fn from(diagnostic: Diagnostic) -> Self {
        let color = match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => ERR_STYLE,
            Some(DiagnosticSeverity::WARNING) => WAR_STYLE,
            _ => ELS_STYLE,
        };
        let actions = process_related_info(diagnostic.related_information);
        Self {
            data: vec![DiagnosticData::new(
                diagnostic.range,
                diagnostic.message,
                color,
                actions,
            )],
        }
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

fn process_related_info(related_info: Option<Vec<DiagnosticRelatedInformation>>) -> Option<Vec<String>> {
    let mut buffer = Vec::new();
    for info in related_info? {
        if info.message.starts_with("consider importing") {
            if let Some(mut imports) = derive_import(&info.message) {
                buffer.append(&mut imports)
            }
        }
    }
    if !buffer.is_empty() {
        return Some(buffer);
    }
    None
}

fn derive_import(message: &str) -> Option<Vec<String>> {
    let matches: Vec<_> = message.match_indices("\n`").map(|(idx, _)| idx).collect();
    let mut buffer = Vec::new();
    let mut end_idx = 0;
    for match_idx in matches {
        let substr = &message[end_idx..match_idx + 1];
        end_idx = match_idx + 2;
        for (current_idx, c) in substr.char_indices().rev() {
            if c == '`' {
                buffer.push(String::from(&substr[current_idx + 1..]));
                break;
            }
        }
    }
    if !buffer.is_empty() {
        return Some(buffer);
    }
    None
}
