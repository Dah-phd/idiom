use crate::{
    configs::{FileType, Theme},
    editor_line::EditorLine,
};
use assert_enum_variants::assert_enum_variants;
use idiom_tui::{UTFSafe, widgets::Writable};

use super::Lang;

impl Lang {
    pub fn is_empty(&self) -> bool {
        self.compl_trigger_chars.is_empty()
            && self.comment_start.is_empty()
            && self.declaration.is_empty()
            && self.key_words.is_empty()
            && self.flow_control.is_empty()
            && self.mod_import.is_empty()
            && self.string_markers.is_empty()
            && self.escape_chars.is_empty()
            && self.completion_data_handler.is_none()
            && self.diagnostic_handler.is_none()
            && self.lang_specific_handler.is_none()
    }
}

pub fn create_text() -> [String; 16] {
    [
        String::from("You can create a String from a literal string with [String::from]:"),
        String::from(""),
        String::from("let hello = String::from(\"Hello, world!\");"),
        String::from(
            "You can append a char to a String with the [push] method, and append a [&str] with the [push_str] method:",
        ),
        String::from(""),
        String::from("let mut hello = String::from(\"Hello, \");"),
        String::from("hello.push('w');"),
        String::from("hello.push_str(\"orld!\");"),
        String::from(
            "If you have a vector of UTF-8 bytes, you can create a String from it with the [from_utf8] method:",
        ),
        String::from("// some bytes, in a vector"),
        String::from("let sparkle_heart = vec![240, 159, 146, 150];"),
        String::from(""),
        String::from("// We know these bytes are valid, so we'll use `unwrap()`."),
        String::from("et sparkle_heart = String::from_utf8(sparkle_heart).unwrap();"),
        String::from(""),
        String::from("assert_eq!(\"💖\", sparkle_heart);"),
    ]
}

#[test]
fn test_stylize() {
    let theme = Theme::default();
    let lang = Lang::from(FileType::Rust);
    let inputs = create_text();
    let styled_lines = inputs.iter().map(|text_line| lang.stylize(text_line, &theme)).enumerate();
    for (idx, sline) in styled_lines {
        assert_eq!(sline.len(), inputs[idx].len());
        assert_eq!(sline.char_len(), inputs[idx].char_len());
        assert_eq!(sline.width(), inputs[idx].width());
        assert_eq!(sline.to_string(), inputs[idx]);
    }
}

#[test]
fn test_completable() {
    let lang = Lang::from(FileType::Rust);
    let line = EditorLine::from("vec.");
    assert!(lang.completable(&line, 4));
    let line = EditorLine::from("vec.push(\"t");
    assert!(!lang.completable(&line, 11));
    let line = EditorLine::from("vec.push(\"text goes here");
    assert!(!lang.completable(&line, 24));
    let line = EditorLine::from("vec.push(\"text goes here\"");
    assert!(!lang.completable(&line, 18));
    let line = EditorLine::from("vec.push(\"text goes here\".");
    assert!(lang.completable(&line, 26));
    let line = EditorLine::from("fn p");
    assert!(!lang.completable(&line, 4));
    let line = EditorLine::from("fn p");
    assert!(!lang.completable(&line, 0));
    let line = EditorLine::from("struct");
    assert!(!lang.completable(&line, 6));
    let line = EditorLine::from("struct Um");
    assert!(!lang.completable(&line, 9));
}

#[test]
fn test_from_file_type_empty() {
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });
    for lang in FileType::iter_langs() {
        assert!(!Lang::from(lang).is_empty());
    }
    assert!(Lang::from(FileType::Text).is_empty());
    assert!(Lang::from(FileType::MarkDown).is_empty());
}
