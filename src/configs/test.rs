use super::{EditorKeyMap, EditorUserKeyMap, FileType};

pub fn mock_editor_key_map() -> EditorKeyMap {
    EditorKeyMap { key_map: EditorUserKeyMap::default().into() }
}

#[test]
fn ensure_file_types_iter() {
    // should be all langs (in this case all FileTypes - 1: ignored type)
    assert_eq!(FileType::iter_langs().len(), 14);
    let ft = FileType::default();
    match ft {
        FileType::Ignored => {}
        FileType::Zig => (),
        FileType::Rust => (),
        FileType::Python => (),
        FileType::TypeScript => (),
        FileType::JavaScript => (),
        FileType::Html => (),
        FileType::Nim => (),
        FileType::C => (),
        FileType::Cpp => (),
        FileType::Yml => (),
        FileType::Toml => (),
        FileType::Lobster => (),
        FileType::Json => (),
        FileType::Shell => (),
    }
}
