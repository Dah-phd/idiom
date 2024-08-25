mod token;

use crate::{
    syntax::{theme::Theme, Legend},
    workspace::line::CodeLine,
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

pub fn set_tokens(tokens: Vec<SemanticToken>, legend: &Legend, theme: &Theme, content: &mut [CodeLine]) {
    let mut tokens = tokens.into_iter();
    let (mut token_line, mut line_idx) = match tokens.next() {
        Some(token) => {
            let line_idx = token.delta_line as usize;
            let token_line = content[line_idx].tokens_mut();
            token_line.clear();
            token_line.push(Token::parse(token, legend, theme));
            (token_line, line_idx)
        }
        None => return,
    };
    for token in tokens {
        if token.delta_line != 0 {
            line_idx += token.delta_line as usize;
            token_line = content[line_idx].tokens_mut();
            token_line.clear();
        };
        token_line.push(Token::parse(token, legend, theme));
    }
}

pub fn set_tokens_partial(
    tokens: Vec<SemanticToken>,
    max_lines: usize,
    legend: &Legend,
    theme: &Theme,
    content: &mut [CodeLine],
) {
    let mut tokens = tokens.into_iter();
    let (mut token_line, mut line_idx) = match tokens.next() {
        Some(token) => {
            let line_idx = token.delta_line as usize;
            if line_idx > max_lines {
                return;
            }
            let token_line = content[line_idx].tokens_mut();
            token_line.clear();
            token_line.push(Token::parse(token, legend, theme));
            (token_line, line_idx)
        }
        None => return,
    };
    for token in tokens {
        if token.delta_line != 0 {
            line_idx += token.delta_line as usize;
            if line_idx > max_lines {
                return;
            }
            token_line = content[line_idx].tokens_mut();
            token_line.clear();
        };
        token_line.push(Token::parse(token, legend, theme));
    }
}
