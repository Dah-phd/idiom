use crate::{configs::IndentConfigs, cursor::CursorPosition, editor_line::EditorLine};
use idiom_tui::UTFSafe;

#[inline(always)]
pub fn insert_clip(clip: &str, content: &mut Vec<EditorLine>, mut cursor: CursorPosition) -> CursorPosition {
    let mut lines = clip.split('\n');
    let first_line = lines.next().expect("first line should exist!");
    let Some(last_line_prefix) = lines.next_back() else {
        content[cursor.line].insert_str(cursor.char, first_line);
        cursor.char += first_line.char_len();
        return cursor;
    };

    let start_line = &mut content[cursor.line];
    let mut end_line = start_line.split_off(cursor.char);
    start_line.push_str(first_line);

    for new_line in lines {
        cursor.line += 1;
        content.insert(cursor.line, new_line.to_owned().into());
    }

    cursor.line += 1;
    cursor.char = last_line_prefix.char_len();

    end_line.insert_str(0, last_line_prefix);
    content.insert(cursor.line, end_line);

    cursor
}

// TODO: refactor - maybe merge partial with insert_lines_indented
/// inserts clip with indent if possible
#[inline(always)]
pub fn insert_lines_try_indented(
    clip: &str,
    cfg: &IndentConfigs,
    content: &mut Vec<EditorLine>,
    mut cursor: CursorPosition,
) -> (String, CursorPosition) {
    let start_indent = &content[cursor.line].get_to(cursor.char).expect("checked within cursor!");
    if !start_indent.trim_start_matches(&cfg.indent).is_empty() {
        return (clip.to_owned(), insert_clip(clip, content, cursor));
    };

    let mut lines = clip.split('\n');
    let first_line = lines.next().expect("first line should exist!");

    let Some(last_line) = lines.next_back().map(|l| l.trim_start()) else {
        content[cursor.line].insert_str(cursor.char, first_line);
        cursor.char += first_line.char_len();
        return (clip.to_owned(), cursor);
    };

    // infering indent if not derived from first line
    let (mut edit_clip, mut indent) = match start_indent.is_empty() {
        true => {
            let indent = cfg.derive_indent_from_lines(&content[..cursor.line]);
            let mut new_clip = indent.to_owned();
            new_clip.push_str(first_line.trim_start());
            (new_clip, indent)
        }
        false => (first_line.trim_start().to_owned(), start_indent.to_string()),
    };

    let start_line = &mut content[cursor.line];
    let end_line = start_line.split_off_string(cursor.char);
    start_line.push_str(&edit_clip);

    for clip_line in lines.map(|l| l.trim_start()) {
        cursor.line += 1;
        if clip_line.is_empty() {
            edit_clip.push('\n');
            content.insert(cursor.line, EditorLine::empty());
            continue;
        }

        let mut indented = indent.to_owned();
        indented.push_str(clip_line);

        let mut new_editor_line = EditorLine::new_posix(indented);
        cfg.unindent_if_before_base_pattern(&mut new_editor_line);
        edit_clip.push('\n');
        edit_clip.push_str(new_editor_line.as_str());
        indent = cfg.derive_indent_from(&new_editor_line);

        content.insert(cursor.line, new_editor_line);
    }

    cursor.line += 1;
    edit_clip.push('\n');
    let mut indented = indent;
    let new_editor_line = if last_line.is_empty() {
        if end_line.starts_with(&indented) || indented.is_empty() {
            cursor.char = indented.char_len();
            EditorLine::new_posix(end_line)
            // ensure indent matches config
        } else if indented.starts_with(&cfg.indent) {
            cursor.char = indented.char_len();
            indented.push_str(&end_line);
            let mut new_editor_line = EditorLine::new_posix(indented);
            let unindent = cfg.unindent_if_before_base_pattern(&mut new_editor_line);
            cursor.char = cursor.char.saturating_sub(unindent);
            if let Some(text) = new_editor_line.get_to(cursor.char) {
                edit_clip.push_str(text);
            }
            new_editor_line
            // backup if current indent does not fit standard indent
        } else {
            cursor.char = indented.char_len();
            if cfg.has_unindent_pattern(&end_line) {
                if cfg.indent.len() > indented.len() {
                    indented.clear();
                } else if let Some((to_range, _)) = indented.char_indices().take(cfg.indent.len() + 1).last() {
                    indented.replace_range(..to_range, "");
                }
            }
            edit_clip.push_str(&indented);
            indented.push_str(&end_line);
            EditorLine::new_posix(indented)
        }
    } else {
        indented.push_str(last_line);
        cursor.char = last_line.char_len(); // set cursor char
        edit_clip.push_str(&indented);
        indented.push_str(&end_line); // push end for insert point
        EditorLine::from(indented)
    };
    content.insert(cursor.line, new_editor_line);
    (edit_clip, cursor)
}

// TODO: refactor - maybe merge partial with insert_lines_try_indented
/// inserts clip with indent
#[inline(always)]
pub fn insert_lines_indented(
    clip: &str,
    cfg: &IndentConfigs,
    content: &mut Vec<EditorLine>,
    mut cursor: CursorPosition,
) -> (String, CursorPosition) {
    let mut lines = clip.split('\n');
    let first_line = lines.next().expect("first line should exist!");

    let Some(last_line) = lines.next_back().map(|l| l.trim_start()) else {
        content[cursor.line].insert_str(cursor.char, first_line);
        cursor.char += first_line.char_len();
        return (clip.to_owned(), cursor);
    };

    // infering indent if not derived from first line
    let (mut edit_clip, mut indent) = match cursor.char {
        0 => {
            let indent = cfg.derive_indent_from_lines(&content[..cursor.line]);
            let mut new_clip = indent.to_owned();
            new_clip.push_str(first_line.trim_start());
            (new_clip, indent)
        }
        pos_char => (
            first_line.trim_start().to_owned(),
            cfg.derive_indent_from_str(content[cursor.line].get_to(pos_char).expect("checked within cursor!")),
        ),
    };

    let start_line = &mut content[cursor.line];
    let end_line = start_line.split_off_string(cursor.char);
    start_line.push_str(&edit_clip);
    for clip_line in lines.map(|l| l.trim_start()) {
        cursor.line += 1;
        if clip_line.is_empty() {
            edit_clip.push('\n');
            content.insert(cursor.line, EditorLine::empty());
            continue;
        }

        let mut indented = indent.to_owned();
        indented.push_str(clip_line);

        let mut new_editor_line = EditorLine::new_posix(indented);
        cfg.unindent_if_before_base_pattern(&mut new_editor_line);
        edit_clip.push('\n');
        edit_clip.push_str(new_editor_line.as_str());
        indent = cfg.derive_indent_from(&new_editor_line);

        content.insert(cursor.line, new_editor_line);
    }

    cursor.line += 1;
    edit_clip.push('\n');
    let mut indented = indent;
    let new_editor_line = if last_line.is_empty() {
        if end_line.starts_with(&indented) || indented.is_empty() {
            cursor.char = indented.char_len();
            EditorLine::new_posix(end_line)
        // ensure indent matches config
        } else if indented.starts_with(&cfg.indent) {
            cursor.char = indented.char_len();
            indented.push_str(&end_line);
            let mut new_editor_line = EditorLine::new_posix(indented);
            let unindent = cfg.unindent_if_before_base_pattern(&mut new_editor_line);
            cursor.char = cursor.char.saturating_sub(unindent);
            if let Some(text) = new_editor_line.get_to(cursor.char) {
                edit_clip.push_str(text);
            }
            new_editor_line
        // backup if current indent does not fit standard indent
        } else {
            cursor.char = indented.char_len();
            if cfg.has_unindent_pattern(&end_line) {
                if cfg.indent.len() > indented.len() {
                    indented.clear();
                } else if let Some((to_range, _)) = indented.char_indices().take(cfg.indent.len() + 1).last() {
                    indented.replace_range(..to_range, "");
                }
            }
            edit_clip.push_str(&indented);
            indented.push_str(&end_line);
            EditorLine::new_posix(indented)
        }
    } else {
        indented.push_str(last_line);
        cursor.char = last_line.char_len(); // set cursor char
        edit_clip.push_str(&indented);
        indented.push_str(&end_line); // push end for insert point
        EditorLine::from(indented)
    };
    content.insert(cursor.line, new_editor_line);
    (edit_clip, cursor)
}

/// panics if out of bounds
#[inline(always)]
pub fn clip_content(from: CursorPosition, to: CursorPosition, content: &mut Vec<EditorLine>) -> String {
    if from.line == to.line {
        let line = &mut content[from.line];
        let clip = line[from.char..to.char].to_owned();
        line.replace_range(from.char..to.char, "");
        return clip;
    };
    let next_line_idx = from.line + 1;
    let clip_init = content[from.line].split_off(from.char).unwrap();
    let clip = content
        .drain(next_line_idx..to.line)
        .fold(clip_init, |clip, next_line| push_on_newline(clip, &next_line.unwrap()));
    let final_clip = content.remove(next_line_idx);
    let (clipped, remaining) = final_clip.split_at(to.char);
    content[from.line].push_str(remaining);
    push_on_newline(clip, clipped)
}

/// panics if range is out of bounds
#[inline(always)]
pub fn remove_content(from: CursorPosition, to: CursorPosition, content: &mut Vec<EditorLine>) {
    if from.line == to.line {
        match content.get_mut(from.line) {
            Some(line) => line.replace_range(from.char..to.char, ""),
            None => content.push(Default::default()),
        };
        return;
    };
    let last_line = content.drain(from.line + 1..=to.line).next_back().expect("Checked above!");
    content[from.line].replace_from(from.char, &last_line[to.char..]);
}

#[inline(always)]
pub fn copy_content(from: CursorPosition, to: CursorPosition, content: &[EditorLine]) -> String {
    if from.line == to.line {
        return content[from.line][from.char..to.char].to_owned();
    };
    let clip_init = content[from.line][from.char..].to_owned();
    let clip = content[from.line + 1..to.line].iter().fold(clip_init, |clip, line| push_on_newline(clip, &line[..]));
    push_on_newline(clip, &content[to.line][..to.char])
}

#[inline(always)]
pub fn get_closing_char(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

#[inline(always)]
pub fn get_closing_char_from_context(ch: char, text: &EditorLine, idx: usize) -> Option<char> {
    match ch {
        '{' if should_close(text.get(idx, idx + 1)) => Some('}'),
        '(' if should_close(text.get(idx, idx + 1)) => Some(')'),
        '[' if should_close(text.get(idx, idx + 1)) => Some(']'),
        '"' | '\''
            if should_close(idx.checked_sub(1).and_then(|p_idx| text.get(p_idx, idx)))
                && should_close(text.get(idx, idx + 1)) =>
        {
            Some(ch)
        }
        _ => None,
    }
}

#[inline(always)]
fn should_close(text: Option<&str>) -> bool {
    let Some(text) = text.and_then(|text| text.chars().next()) else {
        return true;
    };
    text.is_ascii() && !text.is_numeric() && !text.is_alphabetic() && text != '_'
}

#[inline(always)]
pub fn get_opening_char(ch: char) -> Option<char> {
    match ch {
        '}' => Some('{'),
        ')' => Some('('),
        ']' => Some('['),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

#[inline(always)]
pub fn is_scope(first_line: &str, second_line: &str) -> bool {
    let Some(pair) = first_line
        .trim_end()
        .chars()
        .next_back()
        .and_then(|opening| second_line.trim_start().chars().next().map(|closing| (opening, closing)))
    else {
        return false;
    };
    [('{', '}'), ('(', ')'), ('[', ']')].contains(&pair)
}

#[inline(always)]
pub fn is_closing_repeat(line: &EditorLine, ch: char, at: usize) -> bool {
    let Some(opening) = get_opening_char(ch) else {
        return false;
    };
    line[at..].starts_with(ch) && line[..at].contains(opening)
}

#[inline(always)]
pub fn find_line_start(line: &EditorLine) -> usize {
    for (idx, ch) in line.char_indices() {
        if !ch.is_whitespace() {
            return idx;
        }
    }
    0
}

/// simplified push on new line of componenet
#[inline(always)]
fn push_on_newline(mut buf: String, string: &str) -> String {
    buf.push('\n');
    buf.push_str(string);
    buf
}
