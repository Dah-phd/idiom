use super::{token_range_at, Cursor, CursorPosition, Editor};
use crate::{configs::EditorAction, global_state::GlobalState, workspace::actions::perform_tranasaction};

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
        EditorAction::Char(ch) => perform_tranasaction(
            &mut editor.actions,
            &mut editor.lexer,
            &mut editor.content,
            |actions, lexer, content| {
                for cc in editor.multi_positions.iter_mut() {
                    actions.push_char(ch, cc, content, lexer);
                }
            },
        ),
        EditorAction::Backspace => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.backspace(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::Delete => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.del(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::NewLine => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.new_line(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::Indent => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.indent(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::RemoveLine => {
            todo!()
            // editor.select_line();
            // if !editor.cursor.select_is_none() {
            // editor.actions.del(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            // return true;
            // }
        }
        EditorAction::IndentStart => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.indent_start(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::Unintent => {
            perform_tranasaction(
                &mut editor.actions,
                &mut editor.lexer,
                &mut editor.content,
                |actions, lexer, content| {
                    for cursor in editor.multi_positions.iter_mut() {
                        actions.unindent(cursor, content, lexer);
                    }
                },
            );
        }
        EditorAction::SwapUp => {
            todo!("{:?}", &editor.multi_positions);
            // editor.actions.swap_up(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            // return true;
        }
        EditorAction::SwapDown => {
            todo!()
            // editor.actions.swap_down(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            // return true;
        }
        EditorAction::Undo => {
            todo!();
            editor.actions.undo(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::Redo => {
            todo!();
            editor.actions.redo(&mut editor.cursor, &mut editor.content, &mut editor.lexer);
            return true;
        }
        EditorAction::CommentOut => {
            todo!("run only cursors on different lines");
            for cursor in editor.multi_positions.iter_mut() {
                editor.actions.comment_out(
                    editor.file_type.comment_start(),
                    cursor,
                    &mut editor.content,
                    &mut editor.lexer,
                );
            }
        }
        EditorAction::Paste => {
            if let Some(clip) = gs.clipboard.pull() {
                perform_tranasaction(
                    &mut editor.actions,
                    &mut editor.lexer,
                    &mut editor.content,
                    |actions, lexer, content| {
                        for cursor in editor.multi_positions.iter_mut() {
                            actions.paste(clip.to_owned(), cursor, content, lexer);
                        }
                    },
                );
            }
        }
        EditorAction::Cut => {
            todo!();
            if let Some(clip) = editor.cut() {
                gs.clipboard.push(clip);
                return true;
            }
        }
        EditorAction::Copy => {
            todo!();
            if let Some(clip) = editor.copy() {
                gs.clipboard.push(clip);
                return true;
            }
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
        EditorAction::SelectLine => editor.select_line(),
        EditorAction::ScrollUp => editor.cursor.scroll_up(&editor.content),
        EditorAction::ScrollDown => editor.cursor.scroll_down(&editor.content),
        EditorAction::SelectScrollUp => editor.cursor.select_scroll_up(&editor.content),
        EditorAction::SelectScrollDown => editor.cursor.select_scroll_down(&editor.content),
        EditorAction::ScreenUp => editor.cursor.screen_up(&editor.content),
        EditorAction::ScreenDown => editor.cursor.screen_down(&editor.content),
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
        EditorAction::JumpLeft => editor.cursor.jump_left(&editor.content),
        EditorAction::JumpLeftSelect => editor.cursor.jump_left_select(&editor.content),
        EditorAction::JumpRight => editor.cursor.jump_right(&editor.content),
        EditorAction::JumpRightSelect => editor.cursor.jump_right_select(&editor.content),
        EditorAction::EndOfLine => editor.cursor.end_of_line(&editor.content),

        EditorAction::StartOfLine => editor.cursor.start_of_line(&editor.content),
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

pub fn solve_offset(from_to: Option<(CursorPosition, CursorPosition)>) -> Option<CursorPosition> {
    let (from, to) = from_to?;
    None
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

fn sort_cursors(x: &Cursor, y: &Cursor) -> std::cmp::Ordering {
    y.line.cmp(&x.line).then(y.char.cmp(&x.char))
}
