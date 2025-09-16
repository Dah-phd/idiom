use crate::{
    configs::IndentConfigs,
    workspace::{cursor::CursorPosition, line::EditorLine},
};
use idiom_tui::UTF8Safe;
use std::ops::Range;

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

/// inserts clip with indent if possible
#[inline(always)]
pub fn insert_lines_indented(
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
    let (mut new_clip, mut indent) = if start_indent.is_empty() {
        let indent = cfg.derive_indent_from_lines(&content[..cursor.line]);
        (format!("{indent}{}", first_line.trim_start()), indent)
    } else {
        (first_line.trim_start().to_owned(), start_indent.to_string())
    };

    let start_line = &mut content[cursor.line];
    let mut end_line = start_line.split_off(cursor.char);
    start_line.push_str(&new_clip);

    for clip_line in lines.map(|l| l.trim_start()) {
        cursor.line += 1;
        if clip_line.chars().all(|c| c.is_whitespace()) {
            new_clip = push_on_newline(new_clip, "");
            content.insert(cursor.line, EditorLine::empty());
            continue;
        }
        let prefixed = format!("{indent}{clip_line}");
        let mut new_editor_line = EditorLine::from(prefixed);
        cfg.unindent_if_before_base_pattern(&mut new_editor_line);
        new_clip = push_on_newline(new_clip, new_editor_line.as_str());
        indent = cfg.derive_indent_from(&new_editor_line);
        content.insert(cursor.line, new_editor_line);
    }

    cursor.line += 1;
    cursor.char = last_line.char_len() + indent.char_len();

    end_line.insert_str(0, last_line);

    // skipping double indent last line
    if !end_line.starts_with(&indent) {
        end_line.insert_str(0, &indent);
        new_clip = push_on_newline(new_clip, &indent);
    } else {
        new_clip.push('\n');
    }

    new_clip.push_str(last_line);
    content.insert(cursor.line, end_line);

    (new_clip, cursor)
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
    match text.and_then(|text| text.chars().next()) {
        None => true,
        Some(text) => text.is_ascii() && !text.is_numeric() && !text.is_alphabetic() && text != '_',
    }
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
    if let Some(pair) = first_line
        .trim_end()
        .chars()
        .next_back()
        .and_then(|opening| second_line.trim_start().chars().next().map(|closing| (opening, closing)))
    {
        return [('{', '}'), ('(', ')'), ('[', ']')].contains(&pair);
    }
    false
}

#[inline(always)]
pub fn is_closing_repeat(line: &EditorLine, ch: char, at: usize) -> bool {
    if let Some(opening) = get_opening_char(ch) {
        line[at..].starts_with(ch) && line[..at].contains(opening)
    } else {
        false
    }
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

pub fn token_range_at(line: &EditorLine, idx: usize) -> Range<usize> {
    let mut token_start = 0;
    let mut last_not_in_token = false;
    for (char_idx, ch) in line.chars().enumerate() {
        if is_token_char(ch) {
            if last_not_in_token {
                token_start = char_idx;
            }
            last_not_in_token = false;
        } else if char_idx >= idx {
            if last_not_in_token {
                return idx..idx;
            }
            return token_start..char_idx;
        } else {
            last_not_in_token = true;
        }
    }
    if idx < line.char_len() {
        token_start..line.char_len()
    } else if !last_not_in_token && token_start <= idx {
        token_start..idx
    } else {
        idx..idx
    }
}

// finds char indexies
pub fn find_token(text: &str, token: &str) -> Option<(usize, usize)> {
    todo!("function that matches token in string");
    if token.is_empty() {
        return None;
    }
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if !is_token_char(ch) {
            continue;
        }
    }
    None
}

#[inline]
fn is_token_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')
}

#[inline(always)]
fn push_on_newline(mut buf: String, string: &str) -> String {
    buf.push('\n');
    buf.push_str(string);
    buf
}
