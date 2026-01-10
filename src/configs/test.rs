use super::{EditorKeyMap, EditorUserKeyMap, FileFamily, FileType};
use assert_enum_variants::assert_enum_variants;

pub fn mock_editor_key_map() -> EditorKeyMap {
    EditorKeyMap { key_map: EditorUserKeyMap::default().into() }
}

#[test]
fn ensure_file_types_iter() {
    // should be all langs (in this case all FileTypes - 2: ignored type)
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });
    assert_eq!(FileType::iter_langs().len(), 14);
}

#[test]
fn is_code() {
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });

    assert!(!FileType::Text.is_code());
    assert!(!FileType::MarkDown.is_code());
    assert!(FileType::Zig.is_code());
    assert!(FileType::C.is_code());
    assert!(FileType::Cpp.is_code());
    assert!(FileType::Nim.is_code());
    assert!(FileType::Python.is_code());
    assert!(FileType::JavaScript.is_code());
    assert!(FileType::TypeScript.is_code());
    assert!(FileType::Yml.is_code());
    assert!(FileType::Toml.is_code());
    assert!(FileType::Html.is_code());
    assert!(FileType::Lobster.is_code());
    assert!(FileType::Json.is_code());
    assert!(FileType::Shell.is_code());
}

#[test]
fn family() {
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });

    assert_eq!(FileType::Text.family(), FileFamily::Text);
    assert_eq!(FileType::MarkDown.family(), FileFamily::MarkDown);
    assert!(matches!(FileType::Zig.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::C.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Cpp.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Nim.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Python.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::JavaScript.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::TypeScript.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Yml.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Toml.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Html.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Lobster.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Json.family(), FileFamily::Code(..)));
    assert!(matches!(FileType::Shell.family(), FileFamily::Code(..)));
}
