use crate::ext_tui::CrossTerm;
use crate::global_state::IdiomEvent;
use crate::syntax::Lang;
use crate::workspace::line::EditorLine;
use crossterm::style::{Attribute, Attributes, Color, ContentStyle};
use idiom_tui::{Backend, UTF8Safe};
use lsp_types::{DiagnosticRelatedInformation, DiagnosticSeverity};

const ELS_COLOR: Color = Color::DarkGrey;
const ERR_COLOR: Color = Color::Red;
const WAR_COLOR: Color = Color::Yellow;

const ELS_STL: ContentStyle = ContentStyle {
    foreground_color: Some(ELS_COLOR),
    background_color: None,
    attributes: Attributes::none().with(Attribute::Italic),
    underline_color: None,
};

const ERR_STL: ContentStyle = ContentStyle {
    foreground_color: Some(ERR_COLOR),
    background_color: None,
    attributes: Attributes::none().with(Attribute::Italic),
    underline_color: None,
};

const WAR_STL: ContentStyle = ContentStyle {
    foreground_color: Some(WAR_COLOR),
    background_color: None,
    attributes: Attributes::none().with(Attribute::Italic),
    underline_color: None,
};

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
    pub inline_text: String,
    pub message: String,
    pub style: ContentStyle,
    pub severity: DiagnosticSeverity,
    pub info: Option<Vec<DiagnosticRelatedInformation>>,
}

impl DiagnosticData {
    #[inline]
    pub fn text_style(&self) -> ContentStyle {
        self.style
    }

    #[inline]
    pub fn text_color(&self) -> Color {
        self.style.foreground_color.unwrap_or(ELS_COLOR)
    }

    fn new(
        range: lsp_types::Range,
        message: String,
        info: Option<Vec<DiagnosticRelatedInformation>>,
        severity: DiagnosticSeverity,
        style: ContentStyle,
    ) -> Self {
        let inline_text = message.lines().next().map(|s| format!("    {s}")).unwrap_or_default();
        Self {
            start: range.start.character as usize,
            end: if range.start.line == range.end.line { Some(range.end.character as usize) } else { None },
            severity,
            style,
            inline_text,
            message,
            info,
        }
    }
}

pub struct DiagnosticLine {
    data: Vec<DiagnosticData>,
}

impl DiagnosticLine {
    pub fn collect_info(&self, lang: &Lang) -> DiagnosticInfo {
        let mut info = DiagnosticInfo::default();
        let mut buffer = Vec::new();
        for diagnostic in self.data.iter() {
            info.messages.push((diagnostic.message.clone(), diagnostic.text_color()));
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

    pub fn iter(&self) -> std::slice::Iter<'_, DiagnosticData> {
        self.data.iter()
    }

    /// Prints truncated text based on info from diagnostics
    #[inline(always)]
    pub fn inline_render(&self, max_width: usize, backend: &mut CrossTerm) {
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
        self.data.retain(|d| d.severity == DiagnosticSeverity::ERROR);
    }

    pub fn append(&mut self, d: lsp_types::Diagnostic) {
        let severity = d.severity.unwrap_or(DiagnosticSeverity::INFORMATION);
        match severity {
            DiagnosticSeverity::ERROR => {
                let dd = DiagnosticData::new(d.range, d.message, d.related_information, severity, ERR_STL);
                self.data.insert(0, dd);
            }
            DiagnosticSeverity::WARNING => {
                let dd = DiagnosticData::new(d.range, d.message, d.related_information, severity, WAR_STL);
                match self.data.iter().position(|x| x.severity != DiagnosticSeverity::ERROR) {
                    None => self.data.push(dd),
                    Some(index) => self.data.insert(index, dd),
                };
            }
            _ => {
                self.data.push(DiagnosticData::new(d.range, d.message, d.related_information, severity, ELS_STL));
            }
        }
    }
}

impl From<lsp_types::Diagnostic> for DiagnosticLine {
    fn from(diagnostic: lsp_types::Diagnostic) -> Self {
        let severity = diagnostic.severity.unwrap_or(DiagnosticSeverity::INFORMATION);
        let style = match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => ERR_STL,
            Some(DiagnosticSeverity::WARNING) => WAR_STL,
            _ => ELS_STL,
        };
        Self {
            data: vec![DiagnosticData::new(
                diagnostic.range,
                diagnostic.message,
                diagnostic.related_information,
                severity,
                style,
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
