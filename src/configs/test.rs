use std::collections::HashMap;

use super::EditorKeyMap;

pub fn mock_editor_key_map() -> EditorKeyMap {
    EditorKeyMap { key_map: HashMap::default() }
}
