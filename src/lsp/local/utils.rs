use super::{LangStream, PositionedToken};
use crate::workspace::CursorPosition;
use idiom_tui::utils::{UTF8Safe, UTF8SafeStringExt};
use lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensServerCapabilities,
};

pub const NON_TOKEN_ID: u32 = 17;

pub fn utf8_encoder(cursor: lsp_types::Position, content: &[String]) -> CursorPosition {
    let line = cursor.line as usize;
    let mut line_chars = content[line].chars();
    let mut old_char = cursor.character as usize;
    let mut char = 0;
    while let Some(ch_len) = line_chars.next().map(|ch| ch.len_utf8()) {
        if ch_len > old_char {
            break;
        }
        old_char -= ch_len;
        char += 1;
    }
    CursorPosition { line, char }
}

pub fn utf16_encoder(cursor: lsp_types::Position, content: &[String]) -> CursorPosition {
    let line = cursor.line as usize;
    let mut line_chars = content[line].chars();
    let mut old_char = cursor.character as usize;
    let mut char = 0;
    while let Some(ch_len) = line_chars.next().map(|ch| ch.len_utf8().div_ceil(2)) {
        if ch_len > old_char {
            break;
        }
        old_char -= ch_len;
        char += 1;
    }
    CursorPosition { line, char }
}

pub fn utf32_encoder(cursor: lsp_types::Position, _content: &[String]) -> CursorPosition {
    CursorPosition::from(cursor)
}

pub fn swap_content(content: &mut Vec<String>, clip: &str, from: CursorPosition, to: CursorPosition) {
    remove_content(from, to, content);
    insert_clip(clip, content, from);
}

/// panics if range is out of bounds
#[inline(always)]
pub fn remove_content(from: CursorPosition, to: CursorPosition, content: &mut Vec<String>) {
    if from.line == to.line {
        match content.get_mut(from.line) {
            Some(line) => line.utf8_replace_range(from.char..to.char, ""),
            None => content.push(Default::default()),
        };
        return;
    };
    let last_line = content.drain(from.line + 1..=to.line).next_back().expect("Checked above!");
    content[from.line].utf8_replace_from(from.char, last_line.utf8_unsafe_get_from(to.char));
}

#[inline(always)]
pub fn insert_clip(clip: &str, content: &mut Vec<String>, mut cursor: CursorPosition) {
    let mut lines = clip.split('\n').collect::<Vec<_>>();
    if lines.len() == 1 {
        let text = lines[0];
        content[cursor.line].utf8_insert_str(cursor.char, lines[0]);
        cursor.char += text.char_len();
        return;
    };

    let first_line = &mut content[cursor.line];
    let mut last_line = first_line.utf8_split_off(cursor.char);
    first_line.push_str(lines.remove(0));

    let prefix = lines.remove(lines.len() - 1); // len is already checked
    cursor.line += 1;
    cursor.char = prefix.char_len();

    last_line.utf8_insert_str(0, prefix);
    content.insert(cursor.line, last_line);

    for new_line in lines {
        content.insert(cursor.line, new_line.to_owned());
        cursor.line += 1;
    }
}

pub fn full_tokens<T: LangStream>(lsp_tokens: &[Vec<PositionedToken<T>>]) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut last_delta = 0;
    for token_line in lsp_tokens.iter() {
        let mut at_char = 0;
        for token in token_line.iter().filter(stylable_tokens) {
            tokens.push(token.semantic_token(std::mem::take(&mut last_delta), at_char));
            at_char = token.from;
        }
        last_delta += 1;
    }
    tokens
}

pub fn partial_tokens<T: LangStream>(
    lsp_tokens: &[Vec<PositionedToken<T>>],
    start: CursorPosition,
    end: CursorPosition,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut last_delta = start.line as u32;
    let mut remaining = end.line - start.line;
    if remaining == 0 {
        let mut at_char = 0;
        for token in lsp_tokens[start.line].iter().filter(stylable_tokens) {
            if token.from >= start.char && token.from <= end.char {
                tokens.push(token.semantic_token(std::mem::take(&mut last_delta), at_char));
                at_char = token.from;
            }
        }
        return tokens;
    }
    let mut iter = lsp_tokens[start.line..=end.line].iter();
    match iter.next() {
        Some(token_line) => {
            let mut at_char = 0;
            for token in token_line.iter().filter(stylable_tokens).filter(|t| t.from >= start.char) {
                tokens.push(token.semantic_token(std::mem::take(&mut last_delta), at_char));
                at_char = token.from;
            }
            last_delta += 1;
        }
        None => return tokens,
    }
    remaining -= 1;
    while remaining > 0 {
        match iter.next() {
            Some(token_line) => {
                let mut at_char = 0;
                for token in token_line.iter().filter(stylable_tokens) {
                    tokens.push(token.semantic_token(std::mem::take(&mut last_delta), at_char));
                    at_char = token.from;
                }
                last_delta += 1;
            }
            None => return tokens,
        }
        remaining -= 1;
    }
    match iter.next() {
        Some(token_line) => {
            let mut at_char = 0;
            for token in token_line.iter().filter(stylable_tokens).filter(|t| t.from <= end.char) {
                tokens.push(token.semantic_token(std::mem::take(&mut last_delta), at_char));
                at_char = token.from;
            }
        }
        None => return tokens,
    }
    tokens
}

pub fn stylable_tokens<T: LangStream>(token: &&PositionedToken<T>) -> bool {
    token.token_type < NON_TOKEN_ID
}

pub fn create_semantic_capabilities() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        legend: SemanticTokensLegend { token_types: get_local_legend(), token_modifiers: vec![] },
        range: Some(true),
        ..Default::default()
    })
}

pub fn get_local_legend() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::NAMESPACE,      // 0
        SemanticTokenType::TYPE,           // 1
        SemanticTokenType::CLASS,          // 2
        SemanticTokenType::ENUM,           // 3
        SemanticTokenType::INTERFACE,      // 4
        SemanticTokenType::STRUCT,         // 5
        SemanticTokenType::TYPE_PARAMETER, // 6
        SemanticTokenType::PARAMETER,      // 7
        SemanticTokenType::VARIABLE,       // 8
        SemanticTokenType::PROPERTY,       // 9
        SemanticTokenType::FUNCTION,       // 10
        SemanticTokenType::KEYWORD,        // 11
        SemanticTokenType::COMMENT,        // 12
        SemanticTokenType::STRING,         // 13
        SemanticTokenType::NUMBER,         // 14
        SemanticTokenType::DECORATOR,      // 15
        SemanticTokenType::ENUM_MEMBER,    // 16
    ]
}

#[cfg(test)]
mod test {
    use lsp_types::SemanticToken;

    use crate::{
        lsp::local::{tokens::python::PyToken, LangStream, LocalLSP, PositionedToken},
        workspace::CursorPosition,
    };

    use super::{full_tokens, utf16_encoder, utf8_encoder};
    use std::sync::Arc;

    #[test]
    fn test_with_pytoken() {
        let mut pylsp = LocalLSP::<PyToken>::new(Arc::default());
        pylsp.text.push(String::from("class WorkingDirectory:"));
        PyToken::parse(pylsp.text.iter().map(|t| t.as_str()), &mut pylsp.tokens, PositionedToken::<PyToken>::utf32);
        let tokens = full_tokens(&pylsp.tokens);
        assert_eq!(
            tokens,
            vec![
                SemanticToken { delta_line: 0, delta_start: 0, length: 5, token_type: 11, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 16, token_type: 1, token_modifiers_bitset: 0 }
            ]
        );
    }

    #[test]
    fn test_utf8_reposition() {
        let content = vec![String::new(), "t🔥xt".to_owned()];
        let cursor = lsp_types::Position { line: 1, character: 5 };
        assert_eq!(utf8_encoder(cursor, &content), CursorPosition { line: 1, char: 2 })
    }

    #[test]
    fn test_utf16_reposition() {
        let content = vec![String::new(), String::new(), "t🔥xt".to_owned()];
        let cursor = lsp_types::Position { line: 2, character: 3 };
        assert_eq!(utf16_encoder(cursor, &content), CursorPosition { line: 2, char: 2 })
    }
}
