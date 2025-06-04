use super::{EditorKeyMap, EditorUserKeyMap};

pub fn mock_editor_key_map() -> EditorKeyMap {
    EditorKeyMap { key_map: EditorUserKeyMap::default().into() }
}
