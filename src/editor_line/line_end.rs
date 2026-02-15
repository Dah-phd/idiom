use crate::{cursor::CursorPosition, editor_line::EditorLine};
use logos::Logos;

pub const MSDOS_NLINE: &str = "\r\n";
pub const RISCOS_NLINE: &str = "\n\r";
pub const POSIX_NLINE: &str = "\n";
pub const CARRIAGE_NLINE: &str = "\r";

#[derive(Debug, Default)]
pub struct ParsedLines<'a> {
    content: Vec<(&'a str, &'static str)>,
    tabs: Vec<CursorPosition>,
    msdos: Vec<usize>,
    risc_os: Vec<usize>,
    carriage: Vec<usize>,
}

impl ParsedLines<'_> {
    pub fn to_editor_lines(self) -> Vec<EditorLine> {
        self.content.into_iter().map(|(content, line_end)| EditorLine::new(content.into(), line_end)).collect()
    }
}

#[allow(non_camel_case_types)]
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
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MSDOS_NEWLINE => MSDOS_NLINE,
            Self::RISCOS_NEWLINE => RISCOS_NLINE,
            Self::POSIX_NEWLINE => POSIX_NLINE,
            Self::CARRIAGE_NEWLINE => CARRIAGE_NLINE,
            Self::TAB => "\t",
        }
    }

    pub fn split_lines(text: &str) -> ParsedLines<'_> {
        let mut results = ParsedLines::default();
        let mut lines = Self::lexer(text);

        let mut line = 0;
        let mut start = 0;

        while let Some(token) = lines.next() {
            let line_end = match token {
                Err(..) => continue,
                Ok(Self::TAB) => {
                    results.tabs.push(CursorPosition { line, char: 0 });
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

#[cfg(test)]
mod test {
    use super::{LineParser, Logos, CARRIAGE_NLINE, MSDOS_NLINE, POSIX_NLINE, RISCOS_NLINE};

    #[test]
    fn guard_line_parser_as_str() {
        assert_eq!(LineParser::POSIX_NEWLINE.as_str(), POSIX_NLINE);
        assert_eq!(LineParser::MSDOS_NEWLINE.as_str(), MSDOS_NLINE);
        assert_eq!(LineParser::RISCOS_NEWLINE.as_str(), RISCOS_NLINE);
        assert_eq!(LineParser::CARRIAGE_NEWLINE.as_str(), CARRIAGE_NLINE);
    }

    #[test]
    fn test_parse() {
        let text = "a💀w\ndawda\rad";
        let pp = LineParser::split_lines(text);
        panic!("{:?}", pp);
    }

    #[test]
    fn parser_stable() {
        let data = "asd\n\tasdawdaw\n\radwadawdawda\r\nadawdadwd\radwa";
        let tokens = LineParser::lexer(data);
        assert_eq!(
            tokens.collect::<Vec<_>>(),
            [
                Ok(LineParser::POSIX_NEWLINE),
                Ok(LineParser::TAB),
                Ok(LineParser::RISCOS_NEWLINE),
                Ok(LineParser::MSDOS_NEWLINE),
                Ok(LineParser::CARRIAGE_NEWLINE)
            ],
        );
    }
}
