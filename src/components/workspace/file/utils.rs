use super::{CursorPosition, Offset};
use crate::configs::EditorConfigs;
use crate::utils::trim_start_inplace;
use lsp_types::{Position, TextDocumentContentChangeEvent, TextEdit};

pub fn backspace_indent_handler(cfg: &EditorConfigs, line: &mut String, from_idx: usize) -> Offset {
    //! does not handle from_idx == 0
    let prefix = line[..from_idx].trim_start_matches(&cfg.indent);
    if prefix.is_empty() {
        line.replace_range(..cfg.indent.len(), "");
        return Offset::Neg(cfg.indent.len());
    }
    if prefix.chars().all(|c| c.is_whitespace()) {
        let remove_chars_len = prefix.len();
        line.replace_range(from_idx - remove_chars_len..from_idx, "");
        return Offset::Neg(remove_chars_len);
    }
    line.remove(from_idx - 1);
    Offset::Neg(1)
}

pub fn derive_indent_from(cfg: &EditorConfigs, prev_line: &str) -> String {
    let mut indent = prev_line.chars().take_while(|&c| c.is_whitespace()).collect::<String>();
    if let Some(last) = prev_line.trim_end().chars().last() {
        if cfg.indent_after.contains(last) {
            indent.insert_str(0, &cfg.indent);
        }
    };
    indent
}

pub fn indent_from_prev(cfg: &EditorConfigs, prev_line: &str, line: &mut String) -> Offset {
    let indent = derive_indent_from(cfg, prev_line);
    let offset = trim_start_inplace(line) + indent.len();
    line.insert_str(0, &indent);
    offset + unindent_if_before_base_pattern(cfg, line)
}

pub fn unindent_if_before_base_pattern(cfg: &EditorConfigs, line: &mut String) -> Offset {
    if line.starts_with(&cfg.indent) {
        if let Some(first) = line.trim_start().chars().next() {
            if cfg.unindent_before.contains(first) {
                line.replace_range(..cfg.indent.len(), "");
                return Offset::Neg(cfg.indent.len());
            }
        }
    }
    Offset::Pos(0)
}

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
        let mut last_line = to.line;
        while from.line < last_line {
            last_line -= 1;
            if from.line == last_line {
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

pub fn apply_and_rev_edit(edit: &mut TextEdit, content: &mut Vec<String>) {
    let clip = clip_content(&edit.range.start.into(), &edit.range.end.into(), content);
    let new_end = insert_clip(std::mem::replace(&mut edit.new_text, clip), content, edit.range.start.into());
    edit.range.end = Position::new(new_end.line as u32, new_end.char as u32);
}

pub fn into_content_event(edit: &TextEdit) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent { range: Some(edit.range), range_length: None, text: edit.new_text.to_owned() }
}
