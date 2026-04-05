use super::{EditorAction, EditorKeyMap, EditorUserKeyMap, FileFamily, FileType};
use assert_enum_variants::assert_enum_variants;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{collections::HashSet, path::PathBuf};

impl EditorKeyMap {
    pub fn mocked() -> Self {
        EditorKeyMap { key_map: EditorUserKeyMap::default().into() }
    }

    pub fn try_pull(&self, action: &EditorAction) -> Option<KeyEvent> {
        for (k, v) in self.key_map.iter() {
            if v == action {
                return Some(k.clone());
            }
        }
        None
    }
}

#[test]
fn editor_key_map_mock_test() {
    let km = EditorKeyMap::mocked();
    assert!(km.try_pull(&EditorAction::Cancel).is_some());
    let second_call = km.try_pull(&EditorAction::Cancel).unwrap();
    assert_eq!(second_call.code, KeyCode::Esc);
}

#[test]
fn ensure_filt_types_iter_is_unique() {
    // should be all langs (in this case all FileTypes - 2: ignored type)
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });
    let langs = FileType::iter_langs();
    let iter_len = langs.len();
    assert_eq!(iter_len, 14);
    let hash_set = langs.into_iter().collect::<HashSet<_>>();
    assert_eq!(hash_set.len(), iter_len);
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

fn cmp_code_derive_filetype_to_filefamily_from_path(path: &str, file_type: FileType) {
    let path = PathBuf::from(path);
    assert_eq!(FileType::derive_type(path.as_path()), file_type);
    assert_eq!(FileFamily::derive_type(path.as_path()), FileFamily::Code(file_type));
}

#[test]
fn family_derive() {
    assert_enum_variants!(FileType, {
        MarkDown, Text, Zig, Rust, Python, TypeScript, JavaScript, Html, Nim, C, Cpp, Yml, Toml, Lobster, Json, Shell
    });

    let path = PathBuf::from("test.md");
    assert_eq!(FileFamily::derive_type(path.as_path()), FileFamily::MarkDown);
    assert_eq!(FileType::derive_type(path.as_path()), FileType::MarkDown);

    let path = PathBuf::from("any_file");
    assert_eq!(FileFamily::derive_type(path.as_path()), FileFamily::Text);
    assert_eq!(FileType::derive_type(path.as_path()), FileType::Text);

    let path = PathBuf::from("/home/file.txt");
    assert_eq!(FileFamily::derive_type(path.as_path()), FileFamily::Text);
    assert_eq!(FileType::derive_type(path.as_path()), FileType::Text);

    cmp_code_derive_filetype_to_filefamily_from_path("test.rs", FileType::Rust);
    cmp_code_derive_filetype_to_filefamily_from_path("test.zig", FileType::Zig);
    cmp_code_derive_filetype_to_filefamily_from_path("test.c", FileType::C);
    cmp_code_derive_filetype_to_filefamily_from_path("test.cpp", FileType::Cpp);
    cmp_code_derive_filetype_to_filefamily_from_path("test.nim", FileType::Nim);
    cmp_code_derive_filetype_to_filefamily_from_path("test.py", FileType::Python);
    cmp_code_derive_filetype_to_filefamily_from_path("test.pyw", FileType::Python);
    cmp_code_derive_filetype_to_filefamily_from_path("test.js", FileType::JavaScript);
    cmp_code_derive_filetype_to_filefamily_from_path("test.jsx", FileType::JavaScript);
    cmp_code_derive_filetype_to_filefamily_from_path("test.tsx", FileType::TypeScript);
    cmp_code_derive_filetype_to_filefamily_from_path("test.tsx", FileType::TypeScript);
    cmp_code_derive_filetype_to_filefamily_from_path("test.yml", FileType::Yml);
    cmp_code_derive_filetype_to_filefamily_from_path("test.yaml", FileType::Yml);
    cmp_code_derive_filetype_to_filefamily_from_path("test.toml", FileType::Toml);
    cmp_code_derive_filetype_to_filefamily_from_path("test.html", FileType::Html);
    cmp_code_derive_filetype_to_filefamily_from_path("test.lobster", FileType::Lobster);
    cmp_code_derive_filetype_to_filefamily_from_path("test.json", FileType::Json);
    cmp_code_derive_filetype_to_filefamily_from_path("test.sh", FileType::Shell);
    cmp_code_derive_filetype_to_filefamily_from_path(".bashrc", FileType::Shell);
}

#[test]
fn test_editor_key_map_char_mapping() {
    let key_map = EditorKeyMap::mocked();
    let copy = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(Some(EditorAction::Copy), key_map.map(&copy));
    let copy2 = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::CONTROL);
    assert_eq!(Some(EditorAction::Copy), key_map.map(&copy2));

    let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('c')), key_map.map(&c));
    let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('c')), key_map.map(&c));
    let c = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('C')), key_map.map(&c));
    let c = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('C')), key_map.map(&c));

    let sline = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert_eq!(Some(EditorAction::SelectLine), key_map.map(&sline));
    let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('l')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('l')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('L')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('L')), key_map.map(&l));

    let noop = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(None, key_map.map(&noop));
    let l = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('b')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('b')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::NONE);
    assert_eq!(Some(EditorAction::Char('B')), key_map.map(&l));
    let l = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT);
    assert_eq!(Some(EditorAction::Char('B')), key_map.map(&l));
}
