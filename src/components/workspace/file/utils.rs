use std::ops::Range;

use super::CursorPosition;

pub fn insert_clip(clip: String, content: &mut Vec<String>, mut cursor: CursorPosition) -> CursorPosition {
    let mut lines: Vec<_> = clip.split('\n').collect();
    if lines.len() == 1 {
        let text = lines[0];
        content[cursor.line].insert_str(cursor.char, lines[0]);
        cursor.char += text.len();
        cursor
    } else {
        let line = content.remove(cursor.line);
        let (prefix, suffix) = line.split_at(cursor.char);
        let mut first_line = prefix.to_owned();
        first_line.push_str(lines.remove(0));
        content.insert(cursor.line, first_line);
        let last_idx = lines.len() - 1;
        for (idx, select) in lines.iter().enumerate() {
            let next_line = if idx == last_idx {
                let mut last_line = select.to_string();
                cursor.char = last_line.len();
                last_line.push_str(suffix);
                last_line
            } else {
                select.to_string()
            };
            content.insert(cursor.line + 1, next_line);
            cursor.line += 1;
        }
        cursor
    }
}

pub fn clip_content(from: &CursorPosition, to: &CursorPosition, content: &mut Vec<String>) -> String {
    if from.line == to.line {
        let line = &mut content[from.line];
        let clip = line[from.char..to.char].to_owned();
        line.replace_range(from.char..to.char, "");
        clip
    } else {
        let mut clip_vec = vec![content[from.line].split_off(from.char)];
        let mut last_idx = to.line;
        while from.line < last_idx {
            last_idx -= 1;
            if from.line == last_idx {
                let final_clip = content.remove(from.line + 1);
                let (clipped, remaining) = final_clip.split_at(to.char);
                content[from.line].push_str(remaining);
                clip_vec.push(clipped.to_owned())
            } else {
                clip_vec.push(content.remove(from.line + 1))
            }
        }
        clip_vec.join("\n")
    }
}

pub fn remove_content(from: &CursorPosition, to: &CursorPosition, content: &mut Vec<String>) {
    if from.line == to.line {
        if let Some(line) = content.get_mut(from.line) {
            line.replace_range(from.char..to.char, "")
        } else {
            content.push(String::new());
        }
    } else {
        content[from.line].replace_range(from.char.., "");
        let mut last_idx = to.line;
        while from.line < last_idx {
            last_idx -= 1;
            if from.line == last_idx {
                let final_clip = content.remove(from.line + 1);
                content[from.line].push_str(&final_clip[to.char..]);
            } else {
                content.remove(from.line + 1);
            }
        }
    }
}

pub fn copy_content(from: &CursorPosition, to: &CursorPosition, content: &[String]) -> String {
    if from.line == to.line {
        content[from.line][from.char..to.char].to_owned()
    } else {
        let mut at_line = from.line;
        let mut clip_vec = Vec::new();
        clip_vec.push(content[from.line][from.char..].to_owned());
        while at_line < to.line {
            at_line += 1;
            if at_line != to.line {
                clip_vec.push(content[at_line].to_owned())
            } else {
                clip_vec.push(content[at_line][..to.char].to_owned())
            }
        }
        clip_vec.join("\n")
    }
}

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

pub fn is_closing_repeat(line: &str, ch: char, at: usize) -> bool {
    if let Some(opening) = get_opening_char(ch) {
        line[at..].starts_with(ch) && line[..at].contains(opening)
    } else {
        false
    }
}

pub fn find_line_start(line: &str) -> usize {
    for (idx, ch) in line.char_indices() {
        if !ch.is_whitespace() {
            return idx;
        }
    }
    0
}

pub fn token_range_at(line: &str, idx: usize) -> Range<usize> {
    let mut token_start = 0;
    let mut last_not_in_token = false;
    for (char_idx, ch) in line.char_indices() {
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
    if idx < line.len() {
        token_start..line.len()
    } else if !last_not_in_token && token_start <= idx {
        token_start..idx
    } else {
        idx..idx
    }
}
