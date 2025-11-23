use super::{
    apply_multi_cursor_transaction, filter_multi_cursors_per_line_if_no_select, with_new_line_if_not, ControlMap,
};
use crate::{cursor::CursorPosition, editor::Editor, workspace::utils::copy_content};
use lsp_types::TextEdit;
use std::cmp::Ordering;

pub fn insert_import(editor: &mut Editor, insert: String) {
    let offset = editor.actions.insert_top_with_line_offset(insert, &mut editor.content, &mut editor.lexer);
    editor.cursor.add_line_offset(offset);
}

pub fn multic_insert_import(editor: &mut Editor, insert: String) {
    let offset = editor.actions.insert_top_with_line_offset(insert, &mut editor.content, &mut editor.lexer);
    editor.cursor.add_line_offset(offset);
    for cursor in editor.controls.cursors.iter_mut() {
        cursor.add_line_offset(offset);
    }
}

pub fn insert_snippet(editor: &mut Editor, snippet: String, cursor_offset: Option<(usize, usize)>) {
    editor.actions.insert_snippet(&mut editor.cursor, snippet, cursor_offset, &mut editor.content, &mut editor.lexer);
}

pub fn multic_insert_snippet(editor: &mut Editor, snippet: String, cursor_offset: Option<(usize, usize)>) {
    apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
        actions.insert_snippet(cursor, snippet.clone(), cursor_offset, content, lexer);
    })
}

pub fn insert_snippet_with_select(editor: &mut Editor, snippet: String, cursor_offset: (usize, usize), len: usize) {
    editor.actions.insert_snippet_with_select(
        &mut editor.cursor,
        snippet,
        cursor_offset,
        len,
        &mut editor.content,
        &mut editor.lexer,
    );
}

pub fn multic_insert_snippet_with_select(
    editor: &mut Editor,
    snippet: String,
    cursor_offset: (usize, usize),
    len: usize,
) {
    apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
        actions.insert_snippet_with_select(cursor, snippet.clone(), cursor_offset, len, content, lexer);
    });
    // ensure some strange select will not cause cursor collision
    ControlMap::consolidate_cursors(editor);
}

pub fn replace_token(editor: &mut Editor, new: String) {
    editor.actions.replace_token(new, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
}

pub fn multic_replace_token(editor: &mut Editor, new: String) {
    apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
        actions.replace_token(new.clone(), cursor, content, lexer);
    })
}

pub fn cut(editor: &mut Editor) -> Option<String> {
    if editor.content.is_empty() {
        return None;
    }
    Some(editor.actions.cut(&mut editor.cursor, &mut editor.content, &mut editor.lexer))
}

pub fn multic_cut(editor: &mut Editor) -> Option<String> {
    if editor.content.is_empty() {
        return None;
    }
    editor.controls.cursors = filter_multi_cursors_per_line_if_no_select(editor);
    let mut clips = vec![];
    apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
        let clip = actions.cut(cursor, content, lexer);
        clips.push(clip);
    });
    Some(clips.into_iter().rev().map(with_new_line_if_not).collect())
}

pub fn copy(editor: &mut Editor) -> Option<String> {
    if editor.content.is_empty() {
        None
    } else if let Some((from, to)) = editor.cursor.select_get() {
        Some(copy_content(from, to, &editor.content))
    } else {
        Some(format!("{}\n", &editor.content[editor.cursor.line]))
    }
}

pub fn multic_copy(editor: &mut Editor) -> Option<String> {
    if editor.content.is_empty() {
        return None;
    }
    let mut clips = vec![];
    for cursor in editor.controls.cursors.iter() {
        match cursor.select_get() {
            Some((from, to)) => clips.push(copy_content(from, to, &editor.content)),
            None => clips.push(format!("{}\n", &editor.content[cursor.line])),
        }
    }
    Some(clips.into_iter().rev().map(with_new_line_if_not).collect())
}

pub fn paste(editor: &mut Editor, clip: String) {
    editor.actions.paste(clip, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
}

pub fn multic_paste(editor: &mut Editor, clip: String) {
    if editor.controls.cursors.len() == clip.lines().count() {
        let mut clip_lines = clip.lines().rev();
        apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            if let Some(next_clip) = clip_lines.next() {
                actions.paste(next_clip.to_owned(), cursor, content, lexer);
            };
        });
    } else {
        apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.paste(clip.to_owned(), cursor, content, lexer);
        });
    }
}

// SINGLE CURSOR ONLY

pub fn replace_select(editor: &mut Editor, from: CursorPosition, to: CursorPosition, new_clip: &str) {
    ControlMap::ensure_single_cursor(editor);
    editor.actions.replace_select(from, to, new_clip, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
}

pub fn mass_replace(editor: &mut Editor, mut ranges: Vec<(CursorPosition, CursorPosition)>, clip: String) {
    ControlMap::ensure_single_cursor(editor);
    ranges.sort_by(|a, b| {
        let line_ord = b.0.line.cmp(&a.0.line);
        if let Ordering::Equal = line_ord {
            return b.0.char.cmp(&a.0.char);
        }
        line_ord
    });
    editor.actions.mass_replace(&mut editor.cursor, ranges, clip, &mut editor.content, &mut editor.lexer);
}

pub fn apply_file_edits(editor: &mut Editor, mut edits: Vec<TextEdit>) {
    ControlMap::ensure_single_cursor(editor);
    edits.sort_by(|a, b| {
        b.range.start.line.cmp(&a.range.start.line).then(b.range.start.character.cmp(&a.range.start.character))
    });
    editor.actions.apply_edits(&mut editor.cursor, edits, &mut editor.content, &mut editor.lexer);
}
