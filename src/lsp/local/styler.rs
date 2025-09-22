use crate::configs::Theme;
use crate::ext_tui::{StyleExt, StyledLine, Text};
use crate::syntax::Legend;
use idiom_tui::UTF8Safe;

use super::create_semantic_capabilities;
use super::GenericToken;
use super::LangStream;
use super::PositionedToken;

use crossterm::style::ContentStyle;

pub struct Highlighter {
    legend: Legend,
    tokens: Vec<Vec<PositionedToken<GenericToken>>>,
}

impl Highlighter {
    pub fn new(theme: &Theme) -> Self {
        let mut legend = Legend::default();
        legend.map_styles(crate::configs::FileType::Text, theme, &create_semantic_capabilities());
        Self { legend, tokens: vec![] }
    }

    pub fn parse_line(&mut self, text: &str) -> StyledLine {
        GenericToken::parse([text].into_iter(), &mut self.tokens, PositionedToken::<GenericToken>::utf32);
        let text = text.to_owned();
        let mut styled_line = vec![];
        let mut end = 0;
        for pos_token in self.tokens.pop().into_iter().flatten() {
            if end < pos_token.from {
                match text.utf8_get(end, pos_token.from) {
                    Some(chunk) => {
                        styled_line.push(chunk.to_string().into());
                    }
                    None => {
                        if let Some(chunk) = text.utf8_get_from(end) {
                            styled_line.push(chunk.to_string().into());
                        }
                        return styled_line.into();
                    }
                }
            }
            let color = self.legend.parse_to_color(pos_token.token_type as usize, pos_token.modifier);
            match text.utf8_get(pos_token.from, pos_token.from + pos_token.len) {
                Some(chunk) => {
                    styled_line.push(Text::new(chunk.to_string(), Some(ContentStyle::fg(color))));
                    end = pos_token.from + pos_token.len;
                }
                None => {
                    if let Some(chunk) = text.utf8_get_from(pos_token.from) {
                        styled_line.push(Text::new(chunk.to_string(), Some(ContentStyle::fg(color))));
                    }
                    return styled_line.into();
                }
            }
        }
        if let Some(chunk) = text.utf8_get_from(end) {
            styled_line.push(chunk.to_string().into());
        }
        styled_line.into()
    }
}
