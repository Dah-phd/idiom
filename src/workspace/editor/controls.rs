use super::{token_range_at, Cursor, CursorPosition, Editor};
use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    syntax::Lexer,
    workspace::{
        actions::{transaction, Actions},
        utils::copy_content,
        EditorLine,
    },
};

pub fn single_cursor_map(editor: &mut Editor, action: EditorAction, gs: &mut GlobalState) -> bool {
    let (taken, render_update) = editor.lexer.map_modal_if_exists(action, gs);
    if let Some(modal_rect) = render_update {
        editor.updated_rect(modal_rect, gs);
    }
    if taken {
        return true;
    }
    match action {
        // EDITS:
        EditorAction::Char(ch) => {
            editor.actions.push_char(ch, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
            let line = &editor.content[editor.cursor.line];
            if editor.lexer.should_autocomplete(editor.cursor.char, line) {
                let line = line.to_string();
                editor.actions.push_buffer(&mut editor.lexer);
                editor.lexer.get_autocomplete((&editor.cursor).into(), line, gs);
            }
            return true;
        }
        EditorAction::Backspace => {
            editor.actions.backspace(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Delete => {
            editor.actions.del(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::NewLine => {
            editor.actions.new_line(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Indent => {
            editor.actions.indent(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::RemoveLine => {
            editor.select_line();
            if !editor.cursor.select_is_none() {
                editor.actions.del(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
                return true;
            }
        }
        EditorAction::IndentStart => {
            editor.actions.indent_start(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Unintent => {
            editor.actions.unindent(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::SwapUp => {
            editor.actions.swap_up(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::SwapDown => {
            editor.actions.swap_down(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Undo => {
            editor.actions.undo(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Redo => {
            editor.actions.redo(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::CommentOut => {
            editor.actions.comment_out(
                editor.file_type.comment_start(),
                &mut editor.cursor,
                &mut editor.content,
                &mut editor.lexer,
            );
            return true;
        }
        EditorAction::Paste => {
            if let Some(clip) = gs.clipboard.pull() {
                editor.actions.paste(clip, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
                return true;
            }
        }
        EditorAction::Cut => {
            if let Some(clip) = editor.cut() {
                gs.clipboard.push(clip);
                return true;
            }
        }
        EditorAction::Copy => {
            if let Some(clip) = editor.copy() {
                gs.clipboard.push(clip);
                return true;
            }
        }
        // CURSOR:
        EditorAction::Up => editor.cursor.up(&editor.content),
        EditorAction::Down => editor.cursor.down(&editor.content),
        EditorAction::Left => editor.cursor.left(&editor.content),
        EditorAction::Right => editor.cursor.right(&editor.content),
        EditorAction::SelectUp => editor.cursor.select_up(&editor.content),
        EditorAction::SelectDown => editor.cursor.select_down(&editor.content),
        EditorAction::SelectLeft => editor.cursor.select_left(&editor.content),
        EditorAction::SelectRight => editor.cursor.select_right(&editor.content),
        EditorAction::SelectToken => {
            let range = token_range_at(&editor.content[editor.cursor.line], editor.cursor.char);
            if !range.is_empty() {
                editor.cursor.select_set(
                    CursorPosition { line: editor.cursor.line, char: range.start },
                    CursorPosition { line: editor.cursor.line, char: range.end },
                )
            }
        }
        EditorAction::SelectLine => editor.select_line(),
        EditorAction::SelectAll => editor.select_all(),
        EditorAction::ScrollUp => editor.cursor.scroll_up(&editor.content),
        EditorAction::ScrollDown => editor.cursor.scroll_down(&editor.content),
        EditorAction::SelectScrollUp => editor.cursor.select_scroll_up(&editor.content),
        EditorAction::SelectScrollDown => editor.cursor.select_scroll_down(&editor.content),
        EditorAction::ScreenUp => editor.cursor.screen_up(&editor.content),
        EditorAction::ScreenDown => editor.cursor.screen_down(&editor.content),
        EditorAction::NewCursorUp | EditorAction::NewCursorDown | EditorAction::NewCursorWithLine => {
            enable_multi_cursor_mode(editor);
            return editor.map(action, gs);
        }
        EditorAction::JumpLeft => editor.cursor.jump_left(&editor.content),
        EditorAction::JumpLeftSelect => editor.cursor.jump_left_select(&editor.content),
        EditorAction::JumpRight => editor.cursor.jump_right(&editor.content),
        EditorAction::JumpRightSelect => editor.cursor.jump_right_select(&editor.content),
        EditorAction::EndOfLine => editor.cursor.end_of_line(&editor.content),
        EditorAction::EndOfFile => editor.cursor.end_of_file(&editor.content),
        EditorAction::StartOfLine => editor.cursor.start_of_line(&editor.content),
        EditorAction::StartOfFile => editor.cursor.start_of_file(),
        EditorAction::FindReferences => editor.lexer.go_to_reference((&editor.cursor).into(), gs),
        EditorAction::GoToDeclaration => editor.lexer.go_to_declaration((&editor.cursor).into(), gs),
        EditorAction::Help => editor.lexer.help((&editor.cursor).into(), &editor.content, gs),
        EditorAction::LSPRename => {
            let line = &editor.content[editor.cursor.line];
            let token_range = token_range_at(line, editor.cursor.char);
            editor.lexer.start_rename((&editor.cursor).into(), &line[token_range]);
        }
        EditorAction::RefreshUI => editor.lexer.refresh_lsp(gs),
        EditorAction::Save => editor.save(gs),
        EditorAction::Cancel => {
            if editor.cursor.select_take().is_none() {
                editor.actions.push_buffer(&mut editor.lexer);
                return false;
            }
        }
        EditorAction::Close => return false,
    }
    editor.actions.push_buffer(&mut editor.lexer);
    true
}

pub fn multi_cursor_map(editor: &mut Editor, action: EditorAction, gs: &mut GlobalState) -> bool {
    let (taken, render_update) = editor.lexer.map_modal_if_exists(action, gs);
    if let Some(modal_rect) = render_update {
        editor.updated_rect(modal_rect, gs);
    }
    if taken {
        return true;
    }
    match action {
        // EDITS:
        EditorAction::Char(ch) => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.push_char(ch, cursor, content, lexer);
        }),
        EditorAction::Backspace => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.backspace(cursor, content, lexer);
        }),
        EditorAction::Delete => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.del(cursor, content, lexer);
        }),
        EditorAction::NewLine => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.new_line(cursor, content, lexer);
        }),
        EditorAction::Indent => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.indent(cursor, content, lexer);
        }),
        EditorAction::RemoveLine => {
            consolidate_cursors_per_line(editor);
            apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
                actions.del(cursor, content, lexer);
            });
        }
        EditorAction::IndentStart => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.indent_start(cursor, content, lexer);
        }),
        EditorAction::Unintent => apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
            actions.unindent(cursor, content, lexer);
        }),
        EditorAction::SwapUp => {
            let mut last_line = None;
            for cursor in editor.multi_positions.iter_mut().rev() {
                if last_line == Some(cursor.line) {
                    cursor.line += 1;
                    continue;
                }
                last_line = Some(cursor.line);
                editor.actions.swap_up(cursor, &mut editor.content, &mut editor.lexer);
            }
        }
        EditorAction::SwapDown => {
            let mut last_line = None;
            for cursor in editor.multi_positions.iter_mut() {
                if last_line == Some(cursor.line) {
                    cursor.line += 1;
                    continue;
                }
                last_line = Some(cursor.line);
                editor.actions.swap_down(cursor, &mut editor.content, &mut editor.lexer);
            }
        }
        EditorAction::Undo => {
            let text_width = editor.cursor.text_width;
            if let Some(mut cursors) =
                transaction::undo_multi_cursor(&mut editor.actions, &mut editor.content, &mut editor.lexer, text_width)
            {
                let main_index = editor.multi_positions.iter().position(|c| c.max_rows != 0).unwrap_or(usize::MAX);
                if let Some(cursor) = cursors.get_mut(main_index) {
                    cursor.max_rows = editor.cursor.max_rows;
                } else if let Some(cursor) = cursors.last_mut() {
                    cursor.max_rows = editor.cursor.max_rows;
                };
                editor.multi_positions = cursors;
            };
        }
        EditorAction::Redo => {
            let text_width = editor.cursor.text_width;
            if let Some(mut cursors) =
                transaction::redo_multi_cursor(&mut editor.actions, &mut editor.content, &mut editor.lexer, text_width)
            {
                let main_index = editor.multi_positions.iter().position(|c| c.max_rows != 0).unwrap_or(usize::MAX);
                if let Some(cursor) = cursors.get_mut(main_index) {
                    cursor.max_rows = editor.cursor.max_rows;
                } else if let Some(cursor) = cursors.last_mut() {
                    cursor.max_rows = editor.cursor.max_rows;
                };
                editor.multi_positions = cursors;
            }
        }
        EditorAction::CommentOut => {
            let file_type = editor.file_type;
            apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
                actions.comment_out(file_type.comment_start(), cursor, content, lexer);
            });
        }
        EditorAction::Paste => {
            if let Some(clip) = gs.clipboard.pull() {
                if editor.multi_positions.len() == clip.lines().count() {
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
        }
        EditorAction::Cut => {
            if !editor.content.is_empty() {
                editor.multi_positions = filter_multi_cursors_per_line_if_no_select(editor);
                let mut clips = vec![];
                apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
                    let clip = actions.cut(cursor, content, lexer);
                    clips.push(clip);
                });
                gs.clipboard.push(clips.into_iter().rev().map(with_new_line_if_not).collect());
            }
        }
        EditorAction::Copy => {
            if !editor.content.is_empty() {
                let mut clips = vec![];
                for cursor in editor.multi_positions.iter() {
                    match cursor.select_get() {
                        Some((from, to)) => clips.push(copy_content(from, to, &editor.content)),
                        None => clips.push(format!("{}\n", &editor.content[cursor.line])),
                    }
                }
                gs.clipboard.push(clips.into_iter().rev().map(with_new_line_if_not).collect());
            };
        }
        // CURSOR:
        EditorAction::Up => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.up(&editor.content);
            }
        }
        EditorAction::Down => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.down(&editor.content);
            }
        }
        EditorAction::Left => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.left(&editor.content);
            }
        }
        EditorAction::Right => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.right(&editor.content);
            }
        }
        EditorAction::SelectUp => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_up(&editor.content)
            }
        }
        EditorAction::SelectDown => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_down(&editor.content)
            }
        }
        EditorAction::SelectLeft => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_left(&editor.content)
            }
        }
        EditorAction::SelectRight => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_right(&editor.content)
            }
        }
        EditorAction::SelectToken => {
            for cursor in editor.multi_positions.iter_mut() {
                let range = token_range_at(&editor.content[cursor.line], cursor.char);
                if !range.is_empty() {
                    cursor.select_set(
                        CursorPosition { line: cursor.line, char: range.start },
                        CursorPosition { line: cursor.line, char: range.end },
                    )
                }
            }
        }
        EditorAction::SelectLine => {
            for cursor in editor.multi_positions.iter_mut() {
                let start = CursorPosition { line: cursor.line, char: 0 };
                let next_line = cursor.line + 1;
                if editor.content.len() > next_line {
                    cursor.select_set(start, CursorPosition { line: next_line, char: 0 });
                } else {
                    let char = editor.content[start.line].char_len();
                    if char == 0 {
                        continue;
                    };
                    editor.cursor.select_set(start, CursorPosition { line: cursor.line, char });
                };
            }
        }
        EditorAction::ScrollUp => {
            editor.cursor.scroll_up(&editor.content);
            return true;
        }
        EditorAction::ScrollDown => {
            editor.cursor.scroll_down(&editor.content);
            return true;
        }
        EditorAction::SelectScrollUp => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_scroll_up(&editor.content)
            }
        }
        EditorAction::SelectScrollDown => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.select_scroll_down(&editor.content)
            }
        }
        EditorAction::ScreenUp => {
            editor.cursor.screen_up(&editor.content);
            return true;
        }
        EditorAction::ScreenDown => {
            editor.cursor.screen_down(&editor.content);
            return true;
        }
        EditorAction::NewCursorUp => {
            let Some(main_cursor) = editor.multi_positions.last_mut() else {
                restore_single_cursor_mode(editor);
                return true;
            };
            if let Some(new_cursor) = main_cursor.clone_above(&editor.content) {
                editor.multi_positions.push(new_cursor);
            }
        }
        EditorAction::NewCursorDown => {
            let Some(main_cursor) = editor.multi_positions.first_mut() else {
                restore_single_cursor_mode(editor);
                return true;
            };
            if let Some(new_cursor) = main_cursor.clone_below(&editor.content) {
                editor.multi_positions.push(new_cursor);
            }
        }
        EditorAction::NewCursorWithLine => {
            let Some(main_cursor) = editor.multi_positions.first_mut() else {
                restore_single_cursor_mode(editor);
                return true;
            };

            let mut new_cursor = main_cursor.clone();
            editor.actions.new_line_raw(&mut new_cursor, &mut editor.content, &mut editor.lexer);

            if main_cursor.line != new_cursor.line {
                if let Some((from_position, ..)) = main_cursor.select_take() {
                    main_cursor.set_position(from_position);
                }
                editor.multi_positions.push(new_cursor);
            }
        }
        EditorAction::JumpLeft => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.jump_left(&editor.content);
            }
        }
        EditorAction::JumpLeftSelect => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.jump_left_select(&editor.content)
            }
        }
        EditorAction::JumpRight => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.jump_right(&editor.content)
            }
        }
        EditorAction::JumpRightSelect => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.jump_right_select(&editor.content)
            }
        }
        EditorAction::EndOfLine => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.end_of_line(&editor.content)
            }
        }
        EditorAction::StartOfLine => {
            for cursor in editor.multi_positions.iter_mut() {
                cursor.start_of_line(&editor.content)
            }
        }
        EditorAction::FindReferences => {
            todo!();
            editor.lexer.go_to_reference((&editor.cursor).into(), gs)
        }
        EditorAction::GoToDeclaration => {
            todo!();
            editor.lexer.go_to_declaration((&editor.cursor).into(), gs)
        }
        EditorAction::Help => {
            todo!();
            editor.lexer.help((&editor.cursor).into(), &editor.content, gs)
        }
        EditorAction::LSPRename => {
            todo!();
            let line = &editor.content[editor.cursor.line];
            let token_range = token_range_at(line, editor.cursor.char);
            editor.lexer.start_rename((&editor.cursor).into(), &line[token_range]);
        }
        EditorAction::RefreshUI => editor.lexer.refresh_lsp(gs),
        EditorAction::Save => editor.save(gs),
        EditorAction::EndOfFile | EditorAction::StartOfFile | EditorAction::SelectAll => {
            restore_single_cursor_mode(editor);
            return editor.map(action, gs);
        }
        EditorAction::Cancel => {
            restore_single_cursor_mode(editor);
            return true;
        }
        EditorAction::Close => return false,
    }
    consolidate_cursors(editor);
    editor.actions.push_buffer(&mut editor.lexer);
    true
}

pub fn consolidate_cursors_per_line(editor: &mut Editor) {
    let mut idx = 1;
    editor.multi_positions.sort_by(sort_cursors);

    while idx < editor.multi_positions.len() {
        unsafe {
            let [cursor, other] = editor.multi_positions.get_disjoint_unchecked_mut([idx - 1, idx]);
            if cursor.line == other.line {
                cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                editor.multi_positions.remove(idx);
            } else {
                idx += 1;
            }
        }
    }
    if editor.multi_positions.len() < 2 {
        restore_single_cursor_mode(editor);
    }
}

pub fn filter_multi_cursors_per_line_if_no_select(editor: &Editor) -> Vec<Cursor> {
    let mut filtered = vec![];
    let mut index = 0;
    loop {
        let Some(mut cursor) = editor.multi_positions.get(index).cloned() else {
            return filtered;
        };
        if cursor.select_is_none() {
            // remove any cursors already added on the same line
            while let Some(last_filtered) = filtered.last() {
                if last_filtered.line != cursor.line {
                    break;
                }
                cursor.max_rows = std::cmp::max(cursor.max_rows, last_filtered.max_rows);
                filtered.pop();
            }
            // skip all cursors following on the same line
            index += 1;
            while let Some(next_cursor) = editor.multi_positions.get(index) {
                if next_cursor.line != cursor.line {
                    break;
                }
                cursor.max_rows = std::cmp::max(cursor.max_rows, next_cursor.max_rows);
                index += 1;
            }
        } else {
            index += 1;
        };
        filtered.push(cursor);
    }
}

pub fn consolidate_cursors(editor: &mut Editor) {
    let mut idx = 1;

    editor.multi_positions.sort_by(sort_cursors);

    while idx < editor.multi_positions.len() {
        unsafe {
            let [cursor, other] = editor.multi_positions.get_disjoint_unchecked_mut([idx - 1, idx]);
            if cursor.merge_if_intersect(other) {
                cursor.max_rows = std::cmp::max(cursor.max_rows, other.max_rows);
                editor.multi_positions.remove(idx);
            } else {
                idx += 1;
            }
        }
    }
    if editor.multi_positions.len() < 2 {
        restore_single_cursor_mode(editor);
    }
}

pub fn restore_single_cursor_mode(editor: &mut Editor) {
    match editor.multi_positions.iter().find(|c| c.max_rows != 0) {
        Some(cursor) => editor.cursor.set_cursor(cursor),
        None => editor.cursor.set_position(CursorPosition::default()),
    };
    editor.last_render_at_line = None;
    editor.action_map = single_cursor_map;
    editor.multi_positions.clear();
    editor.renderer.single_cursor();
}

pub fn enable_multi_cursor_mode(editor: &mut Editor) {
    editor.multi_positions.clear();
    editor.multi_positions.push(editor.cursor.clone());
    editor.action_map = multi_cursor_map;
    editor.renderer.multi_cursor();
}

fn apply_multi_cursor_transaction<F>(editor: &mut Editor, mut callback: F)
where
    F: FnMut(&mut Actions, &mut Lexer, &mut Vec<EditorLine>, &mut Cursor),
{
    let result: Result<(), ()> = transaction::perform_tranasaction(
        &mut editor.actions,
        &mut editor.lexer,
        &mut editor.content,
        |actions, lexer, content| {
            let mut index = 0;
            let mut last_edit_idx = 0;
            while let Some(cursor) = editor.multi_positions.get_mut(index) {
                (callback)(actions, lexer, content, cursor);

                let current_edit_idx = transaction::check_edit_true_count(actions, lexer);
                if current_edit_idx > last_edit_idx && index > 0 {
                    let edit_offset = transaction::EditOffsetType::get_from_edit(actions, current_edit_idx - 1);
                    edit_offset.apply_cursor(editor.multi_positions.iter_mut().take(index))?;
                };
                last_edit_idx = current_edit_idx;
                index += 1;
            }
            Ok(())
        },
    );

    if result.is_err() {
        // force restore during consolidation of cursors
        editor.multi_positions.retain(|c| c.max_rows != 0);
    }
}

// UTILS

fn sort_cursors(x: &Cursor, y: &Cursor) -> std::cmp::Ordering {
    y.line.cmp(&x.line).then(y.char.cmp(&x.char))
}

fn with_new_line_if_not(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
