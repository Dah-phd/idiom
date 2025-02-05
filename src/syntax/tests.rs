use std::path::PathBuf;

use lsp_types::SemanticToken;

use crate::{configs::FileType, global_state::GlobalState, render::backend::StyleExt, workspace::line::EditorLine};
use crossterm::style::ContentStyle;

use super::{
    lsp_calls::{char_lsp_utf16, char_lsp_utf8, encode_pos_utf16, encode_pos_utf8},
    // theme::Theme,
    tokens::{set_tokens, TokenLine},
    Legend,
    Lexer,
    Token,
};

fn get_text() -> Vec<String> {
    vec![
        "use super::code::CodeLine;".to_owned(),
        "use super::EditorLine;".to_owned(),
        "".to_owned(),
        "#[test]".to_owned(),
        "fn test_insert() {".to_owned(),
        "    let mut line = CodeLine::new(\"text\".to_owned());".to_owned(),
        "    assert!(line.char_len() == 4);".to_owned(),
        "    line.insert(2, 'e');".to_owned(),
        "    assert!(line.is_ascii());".to_owned(),
        "    line.insert(2, '🚀');".to_owned(),
        "    assert!(line.char_len() == 6);".to_owned(),
        "    assert!(!line.is_ascii());".to_owned(),
        "    line.insert(3, 'x');".to_owned(),
        "    assert!(line.char_len() == 7);".to_owned(),
        "    assert!(&line.to_string() == \"te🚀xext\");".to_owned(),
        "}".to_owned(),
    ]
}

fn get_long_line() -> Vec<String> {
    vec![
        "fn get_long_line() -> String {".to_owned(),
        "    let b = \"text🚀text🚀text🚀text🚀text🚀text🚀text🚀text🚀\"\
        .split('🚀').map(|text| text.to_uppercase().to_owned())\
        .map(|mut string| string.push_str(\"text🚀\")).collect::<Vec<_>>();"
            .to_owned(),
        "}".to_owned(),
    ]
}

pub fn longline_token_pair_utf8() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 13, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 16, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 6, token_type: 15, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 17, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 66, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 66, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 5, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 6, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 4, token_type: 12, token_modifiers_bitset: 131074 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 4, token_type: 12, token_modifiers_bitset: 131072 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 12, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 14, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 12, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 6, token_type: 12, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 172040 },
            SemanticToken { delta_line: 0, delta_start: 9, length: 10, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 12, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 7, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 15, token_modifiers_bitset: 8200 },
        ],
        get_long_line(),
    )
}

pub fn longline_token_pair_utf16() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 13, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 16, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 6, token_type: 15, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 17, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 50, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 50, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 5, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 4, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 4, token_type: 12, token_modifiers_bitset: 131074 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 4, token_type: 12, token_modifiers_bitset: 131072 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 12, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 14, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 12, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 6, token_type: 12, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 172040 },
            SemanticToken { delta_line: 0, delta_start: 9, length: 8, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 10, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 7, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 15, token_modifiers_bitset: 8200 },
        ],
        get_long_line(),
    )
}

pub fn longline_token_pair_utf32() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 13, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 16, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 6, token_type: 15, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 17, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 42, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 42, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 5, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 4, token_type: 12, token_modifiers_bitset: 131074 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 4, token_type: 12, token_modifiers_bitset: 131072 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 12, token_type: 8, token_modifiers_bitset: 139272 },
            SemanticToken { delta_line: 0, delta_start: 14, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 3, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 12, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 6, token_type: 12, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 172040 },
            SemanticToken { delta_line: 0, delta_start: 9, length: 7, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 9, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 7, token_type: 8, token_modifiers_bitset: 270600 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 15, token_modifiers_bitset: 8200 },
        ],
        get_long_line(),
    )
}

pub fn create_token_pairs_utf8() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 9, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 10, token_type: 5, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 2, delta_start: 0, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 1, token_modifiers_bitset: 8232 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 11, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 3, token_type: 8, token_modifiers_bitset: 65540 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 6, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 9, token_type: 8, token_modifiers_bitset: 417800 },
            SemanticToken { delta_line: 0, delta_start: 12, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 12, token_type: 14, token_modifiers_bitset: 16384 },
        ],
        get_text(),
    )
}

pub fn create_token_pairs_utf16() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 9, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 10, token_type: 5, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 2, delta_start: 0, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 1, token_modifiers_bitset: 8232 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 11, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 3, token_type: 8, token_modifiers_bitset: 65540 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 4, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 9, token_type: 8, token_modifiers_bitset: 417800 },
            SemanticToken { delta_line: 0, delta_start: 12, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 10, token_type: 14, token_modifiers_bitset: 16384 },
        ],
        get_text(),
    )
}

pub fn create_token_pairs_utf32() -> (Vec<SemanticToken>, Vec<String>) {
    (
        vec![
            SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 9, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 5, token_type: 6, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 10, token_type: 5, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 2, delta_start: 0, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 1, token_modifiers_bitset: 8232 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 21, token_modifiers_bitset: 32 },
            SemanticToken { delta_line: 1, delta_start: 0, length: 2, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 11, token_type: 4, token_modifiers_bitset: 2 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 3, token_type: 6, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32770 },
            SemanticToken { delta_line: 0, delta_start: 5, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 8, token_type: 15, token_modifiers_bitset: 65536 },
            SemanticToken { delta_line: 0, delta_start: 8, length: 2, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 3, token_type: 8, token_modifiers_bitset: 65540 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 6, token_type: 14, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 401416 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 4, token_type: 17, token_modifiers_bitset: 32768 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 6, token_type: 8, token_modifiers_bitset: 425984 },
            SemanticToken { delta_line: 0, delta_start: 7, length: 1, token_type: 10, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 3, length: 3, token_type: 28, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 8, token_type: 8, token_modifiers_bitset: 409600 },
            SemanticToken { delta_line: 0, delta_start: 11, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 10, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 1, delta_start: 4, length: 6, token_type: 7, token_modifiers_bitset: 8200 },
            SemanticToken { delta_line: 0, delta_start: 6, length: 1, token_type: 7, token_modifiers_bitset: 0 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 4, token_type: 17, token_modifiers_bitset: 49152 },
            SemanticToken { delta_line: 0, delta_start: 4, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 9, token_type: 8, token_modifiers_bitset: 417800 },
            SemanticToken { delta_line: 0, delta_start: 12, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 1, length: 1, token_type: 11, token_modifiers_bitset: 16384 },
            SemanticToken { delta_line: 0, delta_start: 2, length: 9, token_type: 14, token_modifiers_bitset: 16384 },
        ],
        get_text(),
    )
}

pub fn zip_text_tokens(text: Vec<String>, tokens: Vec<SemanticToken>) -> Vec<EditorLine> {
    let mut content = text.into_iter().map(EditorLine::new).collect::<Vec<_>>();
    let legend = Legend::default();
    set_tokens(tokens, &legend, &mut content);
    content
}

pub fn mock_utf8_lexer(gs: &mut GlobalState, file_type: FileType) -> Lexer {
    let mut lexer = Lexer::with_context(file_type, PathBuf::new().as_path(), gs);
    lexer.encode_position = encode_pos_utf8;
    lexer.char_lsp_pos = char_lsp_utf8;
    lexer
}

pub fn mock_utf16_lexer(gs: &mut GlobalState, file_type: FileType) -> Lexer {
    let mut lexer = Lexer::with_context(file_type, PathBuf::new().as_path(), gs);
    lexer.encode_position = encode_pos_utf16;
    lexer.char_lsp_pos = char_lsp_utf16;
    lexer
}

pub fn mock_utf32_lexer(gs: &mut GlobalState, file_type: FileType) -> Lexer {
    Lexer::with_context(file_type, PathBuf::new().as_path(), gs)
}

// test tokens
fn create_tokens() -> TokenLine {
    let mut token_line = TokenLine::default();
    token_line.push(Token { len: 3, delta_start: 0, style: ContentStyle::default() });
    token_line.push(Token { len: 4, delta_start: 4, style: ContentStyle::default() });
    token_line
}

#[test]
fn test_token_inc() {
    let mut token_line = create_tokens();
    let mut expected = TokenLine::default();
    expected.push(Token { len: 4, delta_start: 0, style: ContentStyle::default() });
    expected.push(Token { len: 4, delta_start: 5, style: ContentStyle::default() });
    token_line.increment_at(3);
    assert_eq!(token_line, expected);
    token_line.increment_at(5);
    let mut token_line = TokenLine::default();
    token_line.push(Token { len: 3, delta_start: 0, style: ContentStyle::default() });
    token_line.push(Token { len: 5, delta_start: 5, style: ContentStyle::default() });
    assert_eq!(token_line, token_line);
}

#[test]
fn test_token_dec() {
    let mut tl = create_tokens();
    let mut token_line = TokenLine::default();
    token_line.push(Token { len: 3, delta_start: 0, style: ContentStyle::default() });
    token_line.push(Token { len: 4, delta_start: 3, style: ContentStyle::default() });
    tl.decrement_at(3);
    assert_eq!(tl, token_line);

    tl.push(Token { len: 4, delta_start: 5, style: ContentStyle::reversed() });
    let mut token_line = TokenLine::default();
    token_line.push(Token { len: 3, delta_start: 0, style: ContentStyle::default() });
    token_line.push(Token { len: 4, delta_start: 3, style: ContentStyle::reversed() });
    tl.decrement_at(3);
    tl.decrement_at(3);
    tl.decrement_at(3);
    tl.decrement_at(3);
    tl.decrement_at(3);
    assert_eq!(tl, token_line);

    let mut token_line = TokenLine::default();
    token_line.push(Token { len: 1, delta_start: 0, style: ContentStyle::default() });
    token_line.push(Token { len: 4, delta_start: 1, style: ContentStyle::reversed() });
    tl.decrement_at(1);
    tl.decrement_at(1);
    assert_eq!(tl, token_line);
}

#[test]
fn test_token_motions() {
    let mut token_line = TokenLine::default();
    // let text = String::from("tex")
    token_line.push(Token { delta_start: 0, len: 3, style: ContentStyle::default() });
    token_line.push(Token { delta_start: 4, len: 4, style: ContentStyle::default() });
    token_line.push(Token { delta_start: 7, len: 6, style: ContentStyle::reversed() });
    token_line.push(Token { delta_start: 8, len: 4, style: ContentStyle::slowblink() });
    token_line.push(Token { delta_start: 5, len: 5, style: ContentStyle::reversed() });
    token_line.decrement_at(29);
    token_line.decrement_at(28);
    token_line.decrement_at(27);
    token_line.decrement_at(26);
    token_line.decrement_at(25);
    token_line.decrement_at(24);
    // whole token deleted
    assert_eq!(token_line.iter().last().unwrap(), &Token { delta_start: 8, len: 4, style: ContentStyle::slowblink() });
    token_line.decrement_at(23);
    // reaching prev token
    assert_eq!(token_line.iter().last().unwrap(), &Token { delta_start: 8, len: 4, style: ContentStyle::slowblink() });
    token_line.increment_at(23);
    // increased last prev token size
    assert_eq!(token_line.iter().last().unwrap(), &Token { delta_start: 8, len: 5, style: ContentStyle::slowblink() });
    token_line.increment_at(25);
    token_line.increment_at(26);
    // increments passed prev token - so no size change
    assert_eq!(token_line.iter().last().unwrap(), &Token { delta_start: 8, len: 5, style: ContentStyle::slowblink() });
}
