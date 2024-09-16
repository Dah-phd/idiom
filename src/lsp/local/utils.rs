use crate::render::utils::{UTF8Safe, UTF8SafeStringExt};
use crate::workspace::CursorPosition;

// !TODO Dobule check utf8 complience
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
    let last_line = content.drain(from.line + 1..=to.line).last().expect("Checked above!");
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
