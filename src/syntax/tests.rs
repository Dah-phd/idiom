use std::path::PathBuf;

use lsp_types::SemanticToken;

use crate::{configs::FileType, global_state::GlobalState, workspace::line::CodeLine};

use super::{
    lsp_calls::{char_lsp_utf16, char_lsp_utf8, encode_pos_utf16, encode_pos_utf8},
    theme::Theme,
    token::set_tokens,
    Lang, Legend, Lexer,
};

fn create_txt() -> Vec<String> {
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
        create_txt(),
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
        create_txt(),
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
        create_txt(),
    )
}

pub fn zip_text_tokens(text: Vec<String>, tokens: Vec<SemanticToken>) -> Vec<CodeLine> {
    let mut content = text.into_iter().map(CodeLine::new).collect::<Vec<_>>();
    let file_type = FileType::Rust;
    let lang = Lang::from(file_type);
    let legend = Legend::default();
    let theme = Theme::default();
    set_tokens(tokens, &legend, &lang, &theme, &mut content);
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