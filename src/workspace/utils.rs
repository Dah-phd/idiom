use crate::{
    render::UTF8Safe,
    workspace::{cursor::CursorPosition, line::EditorLine},
};
use std::ops::Range;

#[inline(always)]
pub fn insert_clip(clip: &str, content: &mut Vec<EditorLine>, mut cursor: CursorPosition) -> CursorPosition {
    let mut lines = clip.split('\n').collect::<Vec<_>>();
    if lines.len() == 1 {
        let text = lines[0];
        content[cursor.line].insert_str(cursor.char, lines[0]);
        cursor.char += text.char_len();
        return cursor;
    };

    let first_line = &mut content[cursor.line];
    let mut last_line = first_line.split_off(cursor.char);
    first_line.push_str(lines.remove(0));

    let prefix = lines.remove(lines.len() - 1); // len is already checked
    cursor.line += 1;
    cursor.char = prefix.char_len();

    last_line.insert_str(0, prefix);
    content.insert(cursor.line, last_line);

    for new_line in lines {
        content.insert(cursor.line, new_line.to_owned().into());
        cursor.line += 1;
    }

    cursor
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

#[inline(always)]
pub fn token_range_at(line: &EditorLine, idx: usize) -> Range<usize> {
    let mut token_start = 0;
    let mut last_not_in_token = false;
    for (char_idx, ch) in line.chars().enumerate() {
        if ch.is_alphabetic() || ch == '_' {
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

#[inline(always)]
fn push_on_newline(mut buf: String, string: &str) -> String {
    buf.push('\n');
    buf.push_str(string);
    buf
}
