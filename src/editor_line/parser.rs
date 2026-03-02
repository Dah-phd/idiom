use crate::{
    configs::IndentConfigs,
    editor_line::EditorLine,
    error::{IdiomError, IdiomResult},
    global_state::{GlobalState, IdiomEvent},
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

#[derive(Debug, Default)]
pub struct ParsedLines<'a> {
    content: Vec<(&'a str, LineEnd)>,
    tabs: Vec<usize>,
    msdos: bool,
    risc_os: bool,
    carriage: bool,
}

impl ParsedLines<'_> {
    pub fn into_content_or_popup_if_not_formatted(
        self,
        cfg: &IndentConfigs,
        gs: &mut GlobalState,
    ) -> IdiomResult<Vec<EditorLine>> {
        if self.is_formatted() || !self.popup_confirm_sanitize(gs)? {
            return Ok(self.into_editor_lines());
        }
        Ok(self.into_sanitzed_editor_lines(cfg))
    }

    pub fn into_editor_lines(self) -> Vec<EditorLine> {
        self.content.into_iter().map(|(content, line_end)| EditorLine::new(content.into(), line_end)).collect()
    }

    pub fn is_formatted(&self) -> bool {
        self.tabs.is_empty() && !self.msdos && !self.risc_os && !self.carriage
    }

    pub fn into_sanitzed_editor_lines(self, cfg: &IndentConfigs) -> Vec<EditorLine> {
        let ParsedLines { content, tabs, .. } = self;
        if tabs.is_empty() {
            content.into_iter().map(|(content, ..)| EditorLine::new_posix(content.into())).collect()
        } else {
            let size = std::mem::size_of::<EditorLine>();
            let mut lines = Vec::with_capacity(content.len() * size);
            for (line_idx, (text, ..)) in content.into_iter().enumerate() {
                let text = match tabs.contains(&line_idx) {
                    true => text.replace('\t', cfg.indent.as_str()),
                    false => text.to_owned(),
                };
                lines.push(EditorLine::new_posix(text));
            }
            lines
        }
    }

    fn popup_confirm_sanitize(&self, gs: &mut GlobalState) -> IdiomResult<bool> {
        use crossterm::event::{Event, KeyCode, KeyEvent};
        gs.force_screen_rebuild();
        let area = *gs.editor_area();
        area.clear(gs.backend());
        let mut choice = 0;
        self.render_popup(choice, gs);
        loop {
            if crossterm::event::poll(std::time::Duration::from_millis(250))? {
                match crossterm::event::read()? {
                    Event::Resize(width, height) => {
                        gs.event.push(IdiomEvent::Resize { width, height });
                        return Err(IdiomError::GeneralError("File open canceled due to screen resize!".into()));
                    }
                    Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {}
                    Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                        return match choice {
                            0 => Ok(true),
                            1 => Ok(false),
                            _ => Err(IdiomError::GeneralError("File open canceled manually!".into())),
                        };
                    }
                    Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                        choice = choice.checked_sub(1).unwrap_or(2);
                    }
                    Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {
                        choice = match choice > 1 {
                            true => 0,
                            false => choice + 1,
                        };
                    }
                    _ => {}
                }
                self.render_popup(choice, gs);
            }
        }
    }

    fn render_popup(&self, choice: usize, gs: &mut GlobalState) {
        use crate::ext_tui::StyleExt;
        use crossterm::style::ContentStyle;
        use idiom_tui::Backend;

        let mut lines = gs.editor_area().modal_relative(2, 2, 60, 10).into_iter();
        let Some(line) = lines.next() else { return };
        line.render_styled("Found unexpected formatting:", ContentStyle::bold(), gs.backend());
        let Some(line) = lines.next() else { return };
        match self.tabs.is_empty() {
            true => line.render("    Used tabs instead of space indent: N/A", gs.backend()),
            false => line.render_styled(
                "    Used tabs instead of space indent: present!",
                ContentStyle::bold(),
                gs.backend(),
            ),
        };
        let Some(line) = lines.next() else { return };
        match self.msdos {
            true => {
                line.render_styled("    Used MS DOS line end (\\r\\n): present!", ContentStyle::bold(), gs.backend())
            }
            false => line.render("    Used MS DOS line end (\\r\\n): N/A", gs.backend()),
        };
        let Some(line) = lines.next() else { return };
        match self.risc_os {
            true => {
                line.render_styled("    Used Risc OS line end (\\n\\r): present!", ContentStyle::bold(), gs.backend())
            }
            false => line.render("    Used Risc OS line end (\\n\\r): N/A", gs.backend()),
        };
        let Some(line) = lines.next() else { return };
        match self.carriage {
            true => {
                line.render_styled("    Used CARRIAGE line end (\\r): present!", ContentStyle::bold(), gs.backend())
            }
            false => line.render("    Used CARRIAGE line end (\\r): N/A", gs.backend()),
        };
        let Some(line) = lines.next() else { return };
        line.render_styled("Handle choices:", ContentStyle::bold(), gs.backend());
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
        gs.backend().flush_buf();
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
                    results.msdos = true;
                    MSDOS_NLINE
                }
                Ok(Self::RISCOS_NEWLINE) => {
                    results.risc_os = true;
                    RISCOS_NLINE
                }
                Ok(Self::CARRIAGE_NEWLINE) => {
                    results.carriage = true;
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
