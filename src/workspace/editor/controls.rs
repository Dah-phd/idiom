use super::{token_range_at, Cursor, CursorPosition, Editor};
use crate::{configs::EditorAction, global_state::GlobalState};

pub fn single_cursor_map(editor: &mut Editor, action: EditorAction, gs: &mut GlobalState) -> bool {
    let (taken, render_update) = editor.lexer.map_modal_if_exists(action, gs);
    if let Some(modal_rect) = render_update {
        editor.updated_rect(modal_rect, gs);
    }
    if taken {
        return true;
    };
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
            };
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
        EditorAction::NewCursorUp | EditorAction::NewCursorDown => {
            editor.enable_multi_cursors();
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
    };
    match action {
        // EDITS:
        EditorAction::Char(ch) => {
            for cursor in iter_cursors(&mut editor.cursor, &mut editor.positions) {
                editor.actions.push_char(ch, cursor, &mut editor.content, &mut editor.lexer);
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
            };
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
        EditorAction::Up => {
            for cursor in iter_cursors(&mut editor.cursor, &mut editor.positions) {
                cursor.up(&editor.content);
            }
            consolidate_cursors(editor);
        }
        EditorAction::Down => {
            for cursor in iter_cursors(&mut editor.cursor, &mut editor.positions) {
                cursor.down(&editor.content);
            }
            consolidate_cursors(editor);
        }
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
        EditorAction::NewCursorUp => {
            let new_cursor = editor.cursor.clone();
            editor.cursor.up(&editor.content);
            if new_cursor.line != editor.cursor.line {
                editor.positions.push(new_cursor);
            }
        }
        EditorAction::NewCursorDown => {
            let new_cursor = editor.cursor.clone();
            editor.cursor.down(&editor.content);
            if new_cursor.line != editor.cursor.line {
                editor.positions.push(new_cursor);
            }
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

fn iter_cursors<'a>(cursor: &'a mut Cursor, positions: &'a mut Vec<Cursor>) -> impl Iterator<Item = &'a mut Cursor> {
    [cursor].into_iter().chain(positions.iter_mut().rev())
}

fn consolidate_cursors(editor: &mut Editor) {
    if editor.positions.len() < 2 {
        return;
    };

    let mut idx = 1;

    while idx < editor.positions.len() {
        unsafe {
            let [cursor, other] = editor.positions.get_disjoint_unchecked_mut([idx - 1, idx]);
            if cursor.merge_if_intersect(other) {
                editor.positions.remove(idx);
            } else {
                idx += 1;
            };
        };
    }
}
