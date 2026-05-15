use crate::{
    editor_line::EditorLine,
    error::{IdiomError, IdiomResult},
    global_state::GlobalState,
    popups::generic_popup::BasicConfirmPopup,
};
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
pub const VERTICAL_TAB: LineEnd = LineEnd { text: "\u{000B}", char: '⭣' };
pub const FROM_FEED: LineEnd = LineEnd { text: "\u{000C}", char: '▸' };
pub const FILE_END: LineEnd = LineEnd { text: "\u{001C}", char: '◂' };
pub const RECORD_END: LineEnd = LineEnd { text: "\u{001E}", char: '®' };

// RESTRICTED CONTROL CHARS
// they can cause probles with rendering in terminal

const BACKSPACE: char = '\u{0008}';
const ESC: char = '\u{001B}';

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Debug, Logos, PartialEq)]
#[logos(skip r#"[^\r\n\t\u{0008}\u{001B}\u{000B}\u{000C}\u{001C}\u{001E}]"#)]
pub enum LineParser {
    #[token("\r\n")]
    MSDOS_NEWLINE,
    #[token("\n\r")]
    RISCOS_NEWLINE,
    #[token("\n")]
    POSIX_NEWLINE,
    #[token("\r")]
    CARRIAGE_NEWLINE,
    #[token("\u{000B}")]
    VERTICAL_TAB,
    #[token("\u{000C}")]
    FROM_FEED,
    #[token("\u{001C}")]
    FILE_END,
    #[token("\u{001E}")]
    RECORD_END,

    #[token("\t")]
    TAB,

    #[token("\u{0008}")]
    BACKSPACE_CC,
    #[token("\u{001B}")]
    ESC_CC,
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
                Ok(Self::ESC_CC) => {
                    results.replaced_esc_cc.push(line);
                    continue;
                }
                Ok(Self::BACKSPACE_CC) => {
                    results.replaced_backspaces_cc.push(line);
                    continue;
                }
                Ok(Self::VERTICAL_TAB) => {
                    results.non_posix_line_ends = true;
                    VERTICAL_TAB
                }
                Ok(Self::FROM_FEED) => {
                    results.non_posix_line_ends = true;
                    FROM_FEED
                }
                Ok(Self::FILE_END) => {
                    results.non_posix_line_ends = true;
                    FILE_END
                }
                Ok(Self::RECORD_END) => {
                    results.non_posix_line_ends = true;
                    RECORD_END
                }
                Ok(Self::POSIX_NEWLINE) => POSIX_NLINE,
                Ok(Self::MSDOS_NEWLINE) => {
                    results.non_posix_line_ends = true;
                    MSDOS_NLINE
                }
                Ok(Self::RISCOS_NEWLINE) => {
                    results.non_posix_line_ends = true;
                    RISCOS_NLINE
                }
                Ok(Self::CARRIAGE_NEWLINE) => {
                    results.non_posix_line_ends = true;
                    CARRIAGE_NLINE
                }
            };

            let line_end_span = lines.span();
            let text_line = &text[start..line_end_span.start];
            results.content.push((text_line, line_end));
            start = line_end_span.end;
            line += 1;
        }
        results.content.push((&text[start..], POSIX_NLINE));
        results
    }

    pub fn sanitize_text(text: &str, indent: &str) -> String {
        let mut result = text.to_owned();
        let mut replacements = vec![];
        let mut lines = Self::lexer(text);
        while let Some(token) = lines.next() {
            let replacement = match token {
                Err(..) | Ok(Self::POSIX_NEWLINE) => continue,
                Ok(Self::TAB) => (lines.span(), indent),
                Ok(Self::BACKSPACE_CC | Self::ESC_CC) => (lines.span(), ""),
                Ok(
                    Self::MSDOS_NEWLINE
                    | Self::RISCOS_NEWLINE
                    | Self::CARRIAGE_NEWLINE
                    | Self::FROM_FEED
                    | Self::FILE_END
                    | Self::RECORD_END
                    | Self::VERTICAL_TAB,
                ) => (lines.span(), "\n"),
            };
            replacements.push(replacement);
        }
        for (range, replacement) in replacements.into_iter().rev() {
            result.replace_range(range, replacement);
        }
        result
    }
}

#[derive(Default)]
pub struct ParsedLines<'a> {
    content: Vec<(&'a str, LineEnd)>,
    tabs: Vec<usize>,
    /// anything below is unlikely to happen
    non_posix_line_ends: bool,
    replaced_esc_cc: Vec<usize>,
    replaced_backspaces_cc: Vec<usize>,
}

impl ParsedLines<'_> {
    pub fn into_content_or_popup_if_not_formatted(
        self,
        indent: &str,
        gs: &mut GlobalState,
    ) -> IdiomResult<Vec<EditorLine>> {
        if self.is_formatted() {
            return Ok(self.into_editor_lines());
        }
        ConfirmParsedLoading { load_option: LoadingType::Sanitized, parsed: self, indent }.run(gs)
    }

    pub fn into_editor_lines(self) -> Vec<EditorLine> {
        self.transform_to_editor_lines(EditorLine::new)
    }

    pub fn into_sanitzed_editor_lines(mut self, indent: &str) -> Vec<EditorLine> {
        if self.tabs.is_empty() {
            return self.transform_to_editor_lines(|text, _| EditorLine::new_posix(text));
        }
        let tabs = std::mem::take(&mut self.tabs);
        let mut idx = 0;
        self.transform_to_editor_lines(|mut text, _| {
            if tabs.contains(&idx) {
                text = text.replace('\t', indent);
            }
            idx += 1;
            EditorLine::new_posix(text)
        })
    }

    pub fn is_formatted(&self) -> bool {
        self.tabs.is_empty() && !self.non_posix_line_ends
    }

    /// ensures that no restricted control chars are present
    /// removing restricted control chars is unlikely
    fn transform_to_editor_lines(self, mut cb: impl FnMut(String, LineEnd) -> EditorLine) -> Vec<EditorLine> {
        let ParsedLines { content, replaced_esc_cc, replaced_backspaces_cc, .. } = self;
        match (replaced_esc_cc.is_empty(), replaced_backspaces_cc.is_empty()) {
            (true, true) => content.into_iter().map(|(text, end)| (cb)(text.into(), end)).collect(),
            (true, false) => content
                .into_iter()
                .enumerate()
                .map(|(idx, (text, end))| {
                    let mut text = text.to_owned();
                    if replaced_backspaces_cc.contains(&idx) {
                        text.retain(|c| c != BACKSPACE);
                    };
                    (cb)(text, end)
                })
                .collect(),
            (false, true) => content
                .into_iter()
                .enumerate()
                .map(|(idx, (text, end))| {
                    let mut text = text.to_owned();
                    if replaced_esc_cc.contains(&idx) {
                        text.retain(|c| c != ESC);
                    };
                    (cb)(text, end)
                })
                .collect(),
            (false, false) => content
                .into_iter()
                .enumerate()
                .map(|(idx, (text, end))| {
                    let mut text = text.to_owned();
                    match (replaced_esc_cc.contains(&idx), replaced_backspaces_cc.contains(&idx)) {
                        (true, true) => text.retain(|c| c != ESC && c != BACKSPACE),
                        (false, true) => text.retain(|c| c != BACKSPACE),
                        (true, false) => text.retain(|c| c != ESC),
                        (false, false) => (),
                    };
                    (cb)(text, end)
                })
                .collect(),
        }
    }
}

struct ConfirmParsedLoading<'a> {
    parsed: ParsedLines<'a>,
    load_option: LoadingType,
    indent: &'a str,
}

impl<'a> BasicConfirmPopup for ConfirmParsedLoading<'a> {
    type R = Vec<EditorLine>;

    fn render(&mut self, gs: &mut GlobalState) {
        use crate::ext_tui::StyleExt;
        use crossterm::style::{Color, ContentStyle};

        let mut lines = gs.editor_area().modal_relative(2, 2, 60, 12).into_iter();
        let Some(line) = lines.next() else { return };
        line.render_styled("Found unexpected formatting:", ContentStyle::bold(), gs.backend());
        let Some(line) = lines.next() else { return };
        match self.parsed.tabs.is_empty() {
            true => line.render("    Used tabs instead of space indent: N/A", gs.backend()),
            false => line.render_styled(
                "    Used tabs instead of space indent: present!",
                ContentStyle::bold(),
                gs.backend(),
            ),
        };
        let Some(line) = lines.next() else { return };
        match self.parsed.non_posix_line_ends {
            true => line.render_styled("    Used non posix line ends: present!", ContentStyle::bold(), gs.backend()),
            false => line.render("    Used non posix line ends: N/A", gs.backend()),
        };
        let Some(line) = lines.next() else { return };
        line.render_styled("Handle choices:", ContentStyle::bold(), gs.backend());
        let choice = match self.load_option {
            LoadingType::Sanitized => 0,
            LoadingType::LoadAsIs => 1,
            LoadingType::Cancel => 2,
        };
        for (choice_idx, text) in ["sanitize", "do not sanitize - load as is", "cancel"].iter().enumerate() {
            let Some(line) = lines.next() else { return };
            match choice == choice_idx {
                true => {
                    line.render_styled(&format!(" >> {text}"), ContentStyle::reversed(), gs.backend());
                }
                false => {
                    line.render(&format!("    {text}"), gs.backend());
                }
            }
        }
        if !self.parsed.replaced_esc_cc.is_empty() {
            let Some(line) = lines.next() else { return };
            line.render_styled(
                "Found U+001B ESC Control char -> will be stripped from text!",
                ContentStyle::bold().with_fg(Color::Red),
                gs.backend(),
            );
        }
        if !self.parsed.replaced_backspaces_cc.is_empty() {
            let Some(line) = lines.next() else { return };
            line.render_styled(
                "Found U+0008 BACKSPACE Control char -> will be stripped from text!",
                ContentStyle::bold().with_fg(Color::Red),
                gs.backend(),
            );
        }
    }

    fn next_option(&mut self) {
        self.load_option = match self.load_option {
            LoadingType::Sanitized => LoadingType::LoadAsIs,
            LoadingType::LoadAsIs => LoadingType::Cancel,
            LoadingType::Cancel => LoadingType::Sanitized,
        }
    }

    fn prev_option(&mut self) {
        self.load_option = match self.load_option {
            LoadingType::Sanitized => LoadingType::Cancel,
            LoadingType::LoadAsIs => LoadingType::Sanitized,
            LoadingType::Cancel => LoadingType::LoadAsIs,
        }
    }

    fn clear_screen(&self, gs: &mut GlobalState) {
        let rect = *gs.editor_area();
        rect.clear(gs.backend());
    }

    fn cancel_err(&self) -> IdiomError {
        IdiomError::GeneralError("File open canceled manually!".into())
    }

    fn return_select(self) -> IdiomResult<Self::R> {
        match self.load_option {
            LoadingType::Sanitized => Ok(self.parsed.into_sanitzed_editor_lines(self.indent)),
            LoadingType::LoadAsIs => Ok(self.parsed.into_editor_lines()),
            LoadingType::Cancel => Err(self.cancel_err()),
        }
    }
}

#[derive(Default)]
enum LoadingType {
    #[default]
    Sanitized,
    LoadAsIs,
    Cancel,
}
