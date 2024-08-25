mod token;

use crate::{
    render::backend::Style,
    syntax::{theme::Theme, Lang, Legend},
    workspace::line::{CodeLine, EditorLine},
};
use lsp_types::SemanticToken;

pub use token::{Token, TokenLine};

#[derive(Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum TokensType {
    LSP,
    #[default]
    Internal,
}

pub fn set_tokens(tokens: Vec<SemanticToken>, legend: &Legend, lang: &Lang, theme: &Theme, content: &mut [CodeLine]) {
    let mut line_idx = 0;
    let mut char_idx = 0;
    let mut len = 0;
    let mut token_line = TokenLine::default();
    for token in tokens {
        if token.delta_line != 0 {
            len = 0;
            char_idx = 0;
            content[line_idx].replace_tokens(std::mem::take(&mut token_line));
            line_idx += token.delta_line as usize;
        };
        let from = char_idx + token.delta_start as usize;
        let to = from + token.length as usize;
        // enriches the tokens with additinal highlights
        if from.saturating_sub(char_idx + len) > 3 {
            content[line_idx].get(char_idx + len, from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = legend.parse_to_color(token.token_type as usize, token.token_modifiers_bitset, theme);
        token_line.push(Token { from, to, len, delta_start: token.delta_start as usize, style: Style::fg(token_type) });
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}

pub fn set_tokens_partial(
    tokens: Vec<SemanticToken>,
    max_lines: usize,
    legend: &Legend,
    lang: &Lang,
    theme: &Theme,
    content: &mut [CodeLine],
) {
    let mut line_idx = 0;
    let mut char_idx = 0;
    let mut len = 0;
    let mut token_line = TokenLine::default();
    for token in tokens {
        if token.delta_line != 0 {
            len = 0;
            char_idx = 0;
            content[line_idx].replace_tokens(std::mem::take(&mut token_line));
            line_idx += token.delta_line as usize;
            if line_idx > max_lines {
                return;
            }
        };
        let from = char_idx + token.delta_start as usize;
        let to = from + token.length as usize;
        // enriches the tokens with additinal highlights
        if from.saturating_sub(char_idx + len) > 3 {
            content[line_idx].get(char_idx + len, from).inspect(|snippet| {
                Token::enrich(char_idx, lang, theme, snippet, &mut token_line);
            });
        };
        len = token.length as usize;
        let token_type = legend.parse_to_color(token.token_type as usize, token.token_modifiers_bitset, theme);
        token_line.push(Token { from, to, len, delta_start: token.delta_start as usize, style: Style::fg(token_type) });
        char_idx = from;
    }
    if !token_line.is_empty() {
        content[line_idx].replace_tokens(token_line);
    };
}
