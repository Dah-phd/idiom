use crate::{editor_line::EditorLine, popups::generic_popup::PopupChoice};
use logos::Logos;

#[derive(Debug)]
pub struct LineEnd {
    pub text: &'static str,
    pub char: char,
}

pub const MSDOS_NLINE: LineEnd = LineEnd { text: "\r\n", char: '⇆' };
pub const RISCOS_NLINE: LineEnd = LineEnd { text: "\n\r", char: '⇄' };
pub const POSIX_NLINE: LineEnd = LineEnd { text: "\n", char: ' ' };
pub const CARRIAGE_NLINE: LineEnd = LineEnd { text: "\r", char: '←' };

#[derive(Debug, Default)]
pub struct ParsedLines<'a> {
    content: Vec<(&'a str, LineEnd)>,
    tabs: Vec<usize>,
    msdos: Vec<usize>,
    risc_os: Vec<usize>,
    carriage: Vec<usize>,
}

impl ParsedLines<'_> {
    pub fn into_editor_lines(self) -> Vec<EditorLine> {
        self.content.into_iter().map(|(content, line_end)| EditorLine::new(content.into(), line_end)).collect()
    }
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Debug, Logos, PartialEq)]
#[logos(skip r#"[^\r\n\t]"#)]
pub enum LineParser {
    #[token("\r\n")]
    MSDOS_NEWLINE,
    #[token("\n\r")]
    RISCOS_NEWLINE,
    #[token("\n")]
    POSIX_NEWLINE,
    #[token("\r")]
    CARRIAGE_NEWLINE,
    #[token("\t")]
    TAB,
}

impl LineParser {
    pub fn split_lines(text: &str) -> ParsedLines<'_> {
        let mut results = ParsedLines::default();
        let mut lines = Self::lexer(text);

        let mut line = 0;
        let mut start = 0;

        while let Some(token) = lines.next() {
            let line_end = match token {
                Err(..) => continue,
                Ok(Self::TAB) => {
                    results.tabs.push(line);
                    continue;
                }
                Ok(Self::POSIX_NEWLINE) => POSIX_NLINE,
                Ok(Self::MSDOS_NEWLINE) => {
                    results.msdos.push(line);
                    MSDOS_NLINE
                }
                Ok(Self::RISCOS_NEWLINE) => {
                    results.risc_os.push(line);
                    RISCOS_NLINE
                }
                Ok(Self::CARRIAGE_NEWLINE) => {
                    results.carriage.push(line);
                    CARRIAGE_NLINE
                }
            };

            let line_end_span = lines.span();
            results.content.push((&text[start..line_end_span.start], line_end));
            start = line_end_span.end;
            line += 1;
        }
        results.content.push((&text[start..], POSIX_NLINE));
        results
    }
}
