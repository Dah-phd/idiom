use crate::{
    configs::IndentConfigs,
    ext_tui::CrossTerm,
    workspace::{cursor::CursorPosition, line::EditorLine},
};
use idiom_tui::{
    widgets::{Text, Writable},
    UTF8Safe,
};
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

/// returns range of char positions, not indexies
///  - that means text.chars().nth(range.start) will return the start position,
///    instead of text[range.start]
pub fn word_range_at(line: &EditorLine, idx: usize) -> Range<usize> {
    let mut token_start = 0;
    let mut last_not_in_token = false;
    for (char_idx, ch) in line.chars().enumerate() {
        if is_word_char(ch) {
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

pub fn find_words_inline_from<'a>(
    from: CursorPosition,
    content: &'a [EditorLine],
    word: &'a Text<CrossTerm>,
) -> Option<impl Iterator<Item = (CursorPosition, CursorPosition)> + use<'a>> {
    let text = content.get(from.line)?;
    let skipped = text.get_to(from.char)?;
    let char_before_heystack = skipped.chars().next_back();
    let heystack = &text.as_str()[skipped.len()..];
    Some(heystack.match_indices(word.as_str()).flat_map(move |(position, _)| {
        let prefix = &heystack[..position];
        let prev_char = if position == 0 { char_before_heystack } else { prefix.chars().next_back() };
        if prev_char.map(is_word_char).unwrap_or_default() {
            return None;
        };
        let end_char_idx = position + word.len();
        if heystack[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
            return None;
        };
        if text.is_simple() {
            return Some((
                CursorPosition { line: from.line, char: from.char + position },
                CursorPosition { line: from.line, char: from.char + end_char_idx },
            ));
        }
        let char = from.char + prefix.char_len();
        Some((
            CursorPosition { line: from.line, char },
            CursorPosition { line: from.line, char: char + word.char_len() },
        ))
    }))
}

pub fn find_words_inline_to<'a>(
    to: CursorPosition,
    content: &'a [EditorLine],
    word: &'a Text<CrossTerm>,
) -> Option<impl Iterator<Item = (CursorPosition, CursorPosition)> + use<'a>> {
    let text = content.get(to.line)?;
    let heystack = text.get_to(to.char)?;
    Some(heystack.match_indices(word.as_str()).flat_map(move |(position, _)| {
        let prefix = &heystack[..position];
        if prefix.chars().next_back().map(is_word_char).unwrap_or_default() {
            return None;
        };
        let end_char_idx = position + word.len();
        if text.as_str()[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
            return None;
        };
        if text.is_simple() {
            return Some((
                CursorPosition { line: to.line, char: position },
                CursorPosition { line: to.line, char: end_char_idx },
            ));
        }
        let char = prefix.char_len();
        Some((CursorPosition { line: to.line, char }, CursorPosition { line: to.line, char: char + word.char_len() }))
    }))
}

/// maps token selects for iter <(line_idx, EditorLine)>
pub fn iter_word_selects<'a, B>(
    content_iter: B,
    word: &'a Text<CrossTerm>,
) -> impl Iterator<Item = (CursorPosition, CursorPosition)> + use<'a, B>
where
    B: Iterator<Item = (usize, &'a EditorLine)>,
{
    content_iter.flat_map(move |(line, text)| {
        text.as_str().match_indices(word.as_str()).flat_map(move |(position, _)| {
            let prefix = &text.as_str()[..position];
            if prefix.chars().next_back().map(is_word_char).unwrap_or_default() {
                return None;
            }
            let end_char_idx = position + word.len();
            if text.as_str()[end_char_idx..].chars().next().map(is_word_char).unwrap_or_default() {
                return None;
            };
            if text.is_simple() {
                return Some((CursorPosition { line, char: position }, CursorPosition { line, char: end_char_idx }));
            }
            let char = prefix.char_len();
            Some((CursorPosition { line, char }, CursorPosition { line, char: char + word.char_len() }))
        })
    })
}

#[inline]
fn is_word_char(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')
}

#[inline(always)]
fn push_on_newline(mut buf: String, string: &str) -> String {
    buf.push('\n');
    buf.push_str(string);
    buf
}
