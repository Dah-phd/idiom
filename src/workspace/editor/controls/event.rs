use super::{
    apply_multi_cursor_transaction, consolidate_cursors, consolidate_cursors_per_line, multi_cursor_word_select,
    ControlMap,
};
use crate::{
    configs::EditorAction,
    global_state::GlobalState,
    workspace::{
        actions::transaction,
        cursor::{CursorPosition, PositionedWord, WordRange},
        editor::{Editor, EditorModal},
    },
};

pub fn single_cursor_map(editor: &mut Editor, action: EditorAction, gs: &mut GlobalState) -> bool {
    let (taken, render_update) = EditorModal::map_if_exists(editor, action, gs);
    if let Some(modal_rect) = render_update {
        editor.clear_lines_cache(modal_rect, gs);
    }
    if taken {
        return true;
    }
    match action {
        // EDITS:
        EditorAction::Char(ch) => {
            editor.actions.push_char(ch, &mut editor.cursor, &mut editor.content, &mut editor.lexer);
            let line = &editor.content[editor.cursor.line];
            if !editor.modal.is_autocomplete() && editor.lexer.should_autocomplete(editor.cursor.char, line) {
                let line = line.to_string();
                editor.actions.push_buffer(&mut editor.lexer);
                editor.lexer.get_autocomplete(editor.cursor.get_position(), line, gs);
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
                (editor.controls.paste)(editor, clip);
                return true;
            }
        }
        EditorAction::Cut => {
            if let Some(clip) = (editor.controls.cut)(editor) {
                gs.clipboard.push(clip);
                return true;
            }
        }
        EditorAction::Copy => {
            if let Some(clip) = (editor.controls.copy)(editor) {
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
            if let Some(range) = WordRange::find_at(&editor.content, editor.cursor.get_position()) {
                let (from, to) = range.as_select();
                if editor.cursor.select_get() == Some((from, to)) && ControlMap::try_multi_cursor(editor) {
                    return editor.map(action, gs);
                } else {
                    editor.cursor.select_set(from, to);
                }
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
            if ControlMap::try_multi_cursor(editor) {
                return editor.map(action, gs);
            };
        }
        EditorAction::JumpLeft => editor.cursor.jump_left(&editor.content),
        EditorAction::JumpLeftSelect => editor.cursor.jump_left_select(&editor.content),
        EditorAction::JumpRight => editor.cursor.jump_right(&editor.content),
        EditorAction::JumpRightSelect => editor.cursor.jump_right_select(&editor.content),
        EditorAction::EndOfLine => editor.cursor.end_of_line(&editor.content),
        EditorAction::EndOfFile => editor.cursor.end_of_file(&editor.content),
        EditorAction::StartOfLine => editor.cursor.start_of_line(&editor.content),
        EditorAction::StartOfFile => editor.cursor.start_of_file(),
        EditorAction::IdiomCommand => return false,
        EditorAction::FindReferences => editor.lexer.go_to_reference((&editor.cursor).into(), gs),
        EditorAction::GoToDeclaration => editor.lexer.go_to_declaration((&editor.cursor).into(), gs),
        EditorAction::Help => {
            let position = editor.cursor.get_position();
            if let Some(actions) = editor.content[position.line].diagnostic_info(&editor.lexer.lang) {
                editor.modal.replace_with_action(actions);
            }
            editor.lexer.help(position, gs)
        }
        EditorAction::LSPRename => {
            let position = editor.cursor.get_position();
            editor.modal.start_renames(&editor.content, position);
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
    let (taken, render_update) = EditorModal::map_if_exists(editor, action, gs);
    if let Some(modal_rect) = render_update {
        editor.clear_lines_cache(modal_rect, gs);
    }
    if taken {
        return true;
    }
    match action {
        // EDITS:
        EditorAction::Char(ch) => {
            let modal_is_autocomplete = editor.modal.is_autocomplete();
            let mut auto_complete = None;
            apply_multi_cursor_transaction(editor, |actions, lexer, content, cursor| {
                actions.push_char(ch, cursor, content, lexer);
                if cursor.max_rows != 0 {
                    let line = &content[cursor.line];
                    if !modal_is_autocomplete && lexer.should_autocomplete(cursor.char, line) {
                        auto_complete = Some((cursor.get_position(), line.to_string()));
                    }
                }
            });
            if let Some((position, line)) = auto_complete {
                editor.cursor.set_position(position);
                editor.lexer.get_autocomplete(position, line, gs);
            }
        }
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
            for cursor in editor.controls.cursors.iter_mut().rev() {
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
            for cursor in editor.controls.cursors.iter_mut() {
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
                let main_index = editor.controls.cursors.iter().position(|c| c.max_rows != 0).unwrap_or(usize::MAX);
                if let Some(cursor) = cursors.get_mut(main_index) {
                    cursor.max_rows = editor.cursor.max_rows;
                } else if let Some(cursor) = cursors.last_mut() {
                    cursor.max_rows = editor.cursor.max_rows;
                };
                editor.controls.cursors = cursors;
            };
        }
        EditorAction::Redo => {
            let text_width = editor.cursor.text_width;
            if let Some(mut cursors) =
                transaction::redo_multi_cursor(&mut editor.actions, &mut editor.content, &mut editor.lexer, text_width)
            {
                let main_index = editor.controls.cursors.iter().position(|c| c.max_rows != 0).unwrap_or(usize::MAX);
                if let Some(cursor) = cursors.get_mut(main_index) {
                    cursor.max_rows = editor.cursor.max_rows;
                } else if let Some(cursor) = cursors.last_mut() {
                    cursor.max_rows = editor.cursor.max_rows;
                };
                editor.controls.cursors = cursors;
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
                (editor.controls.paste)(editor, clip);
            }
        }
        EditorAction::Cut => {
            if let Some(clip) = (editor.controls.cut)(editor) {
                gs.clipboard.push(clip);
            }
        }
        EditorAction::Copy => {
            if let Some(clip) = (editor.controls.copy)(editor) {
                gs.clipboard.push(clip);
                return true;
            }
        }
        // CURSOR:
        EditorAction::Up => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.up(&editor.content);
            }
        }
        EditorAction::Down => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.down(&editor.content);
            }
        }
        EditorAction::Left => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.left(&editor.content);
            }
        }
        EditorAction::Right => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.right(&editor.content);
            }
        }
        EditorAction::SelectUp => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.select_up(&editor.content)
            }
        }
        EditorAction::SelectDown => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.select_down(&editor.content)
            }
        }
        EditorAction::SelectLeft => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.select_left(&editor.content)
            }
        }
        EditorAction::SelectRight => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.select_right(&editor.content)
            }
        }
        EditorAction::SelectToken => {
            let maybe_word = editor.controls.cursors.first().and_then(|cursor| {
                let current_select = cursor.select_get()?;
                let word = PositionedWord::find_at(&editor.content, cursor.get_position())?;
                if word.range().as_select() != current_select {
                    return None;
                }
                editor
                    .controls
                    .cursors
                    .iter()
                    .skip(1)
                    .all(|c| {
                        let Some((from, to)) = c.select_get() else { return false };
                        if from.line != to.line || from.char == to.char {
                            return false;
                        };
                        editor.content[from.line].get(from.char, to.char) == Some(word.as_str())
                    })
                    .then_some(word)
            });

            match maybe_word {
                Some(word) => multi_cursor_word_select(editor, word),
                None => {
                    for cursor in editor.controls.cursors.iter_mut() {
                        let Some(range) = WordRange::find_at(&editor.content, cursor.get_position()) else {
                            continue;
                        };
                        let (from, to) = range.as_select();
                        cursor.select_set(from, to)
                    }
                }
            }
        }
        EditorAction::SelectLine => {
            for cursor in editor.controls.cursors.iter_mut() {
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
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.select_scroll_up(&editor.content)
            }
        }
        EditorAction::SelectScrollDown => {
            for cursor in editor.controls.cursors.iter_mut() {
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
            let Some(main_cursor) = editor.controls.cursors.last_mut() else {
                ControlMap::single_cursor(editor);
                return true;
            };
            if let Some(new_cursor) = main_cursor.clone_above(&editor.content) {
                editor.controls.cursors.push(new_cursor);
            }
        }
        EditorAction::NewCursorDown => {
            let Some(main_cursor) = editor.controls.cursors.first_mut() else {
                ControlMap::single_cursor(editor);
                return true;
            };
            if let Some(new_cursor) = main_cursor.clone_below(&editor.content) {
                editor.controls.cursors.insert(0, new_cursor);
            }
        }
        EditorAction::NewCursorWithLine => {
            let Some(main_cursor) = editor.controls.cursors.first_mut() else {
                ControlMap::single_cursor(editor);
                return true;
            };

            let mut new_cursor = main_cursor.clone();
            editor.actions.new_line_raw(&mut new_cursor, &mut editor.content, &mut editor.lexer);

            if main_cursor.line != new_cursor.line {
                if let Some((from_position, ..)) = main_cursor.select_take() {
                    main_cursor.set_position(from_position);
                }
                new_cursor.max_rows = 0;
                editor.controls.cursors.push(new_cursor);
            }
        }
        EditorAction::JumpLeft => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.jump_left(&editor.content);
            }
        }
        EditorAction::JumpLeftSelect => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.jump_left_select(&editor.content)
            }
        }
        EditorAction::JumpRight => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.jump_right(&editor.content)
            }
        }
        EditorAction::JumpRightSelect => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.jump_right_select(&editor.content)
            }
        }
        EditorAction::EndOfLine => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.end_of_line(&editor.content)
            }
        }
        EditorAction::StartOfLine => {
            for cursor in editor.controls.cursors.iter_mut() {
                cursor.start_of_line(&editor.content)
            }
        }
        EditorAction::IdiomCommand => return false,
        EditorAction::RefreshUI => editor.lexer.refresh_lsp(gs),
        EditorAction::Save => editor.save(gs),
        EditorAction::EndOfFile
        | EditorAction::StartOfFile
        | EditorAction::SelectAll
        | EditorAction::FindReferences
        | EditorAction::GoToDeclaration
        | EditorAction::LSPRename
        | EditorAction::Help => {
            ControlMap::single_cursor(editor);
            return editor.map(action, gs);
        }
        EditorAction::Cancel => {
            ControlMap::single_cursor(editor);
            return true;
        }
        EditorAction::Close => return false,
    }
    consolidate_cursors(editor);
    editor.actions.push_buffer(&mut editor.lexer);
    true
}
