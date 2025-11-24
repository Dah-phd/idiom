use crate::{
    cursor::{CursorPosition, EncodedWordRange, PositionedWord},
    editor::syntax::{tokens::TokenLine, Lexer},
    editor::Editor,
    embeded_term::EditorTerminal,
    ext_tui::StyleExt,
    global_state::GlobalState,
    tree::Tree,
    workspace::Workspace,
};
use crossterm::{
    self,
    event::{poll, read, Event},
    style::{Attribute, Attributes, ContentStyle},
};
use idiom_tui::Backend;
use std::time::Duration;

const FRAME_RATE: Duration = Duration::from_millis(250);
const UNFOCUSSED_FRAME_RAGE: Duration = Duration::from_secs(5);
const STYLE_BASE: ContentStyle = ContentStyle {
    attributes: Attributes::none().with(Attribute::Bold).with(Attribute::Underlined).with(Attribute::Italic),
    background_color: None,
    foreground_color: None,
    underline_color: None,
};

/// Not a real popup
/// uses similar structure to show marked word
/// but does not implement all APIs
pub fn render_marked_word(
    gs: &mut GlobalState,
    ws: &mut Workspace,
    tree: &mut Tree,
    term: &mut EditorTerminal,
) -> crate::error::IdiomResult<()> {
    let Some(editor) = ws.get_active() else { return Ok(()) };
    if !editor.file_type.is_code() {
        return Ok(());
    };

    let Some(ranges) = find_ranges(editor) else {
        return Ok(());
    };
    perform_render(editor, &ranges, gs);

    let mut frame_rate = Duration::from_millis(250);

    loop {
        if poll(frame_rate)? {
            match read()? {
                Event::Key(event) => {
                    clear_marked_cache(editor, ranges);
                    _ = gs.map_key(&event, ws, tree, term);
                    return Ok(());
                }
                Event::Mouse(event) => {
                    clear_marked_cache(editor, ranges);
                    gs.map_mouse(event, tree, ws, term);
                    return Ok(());
                }
                Event::Paste(clip) => {
                    clear_marked_cache(editor, ranges);
                    gs.passthrough_paste(clip, ws, term);
                    return Ok(());
                }
                Event::Resize(width, height) => {
                    clear_marked_cache(editor, ranges);
                    gs.full_resize(ws, term, width, height);
                    return Ok(());
                }
                Event::FocusGained => {
                    frame_rate = FRAME_RATE;
                }
                Event::FocusLost => {
                    frame_rate = UNFOCUSSED_FRAME_RAGE;
                }
            }
        }
        Lexer::context(editor, gs);
        if !editor.has_render_cache() {
            let Some(new_ranges) = find_ranges(editor) else {
                return Ok(());
            };
            if ranges != new_ranges {
                return Ok(());
            }
            perform_render(editor, &ranges, gs);
        }
    }
}

fn perform_render(editor: &mut Editor, ranges: &[EncodedWordRange], gs: &mut GlobalState) {
    let style = STYLE_BASE.with_fg(gs.ui_theme.accent());
    let mut stored_tokens: Vec<(usize, TokenLine)> = vec![];
    for word in ranges {
        let range_line = word.line();
        let line = &mut editor.unsafe_content_mut()[range_line];
        if stored_tokens.iter().any(|(line, _)| line == &range_line) {
            line.tokens_mut_unchecked().set_encoded_word_checked(word, style);
        } else {
            let mut new_tokens = line.tokens().clone();
            new_tokens.set_encoded_word_checked(word, style);
            stored_tokens.push((range_line, std::mem::replace(line.tokens_mut(), new_tokens)));
        }
    }

    gs.backend.freeze();
    editor.render(gs);
    gs.backend.flush_buf();
    gs.backend.unfreeze();

    for (idx, tokens) in stored_tokens {
        *editor.unsafe_content_mut()[idx].tokens_mut_unchecked() = tokens;
    }
}

fn clear_marked_cache(editor: &mut Editor, ranges: Vec<EncodedWordRange>) {
    for range in ranges {
        editor.unsafe_content_mut()[range.line()].cached.reset();
    }
}

fn find_ranges(editor: &Editor) -> Option<Vec<EncodedWordRange>> {
    let position = editor.controls().get_base_cursor_position().unwrap_or(editor.cursor().get_position());
    if let Some(ranges) = try_find_brackets(editor, position) {
        return Some(ranges);
    }
    let Some(word) = PositionedWord::find_at(editor.content(), position) else {
        if position.char == 0 {
            return None;
        }
        let prev_position = CursorPosition { line: position.line, char: position.char + 1 };
        return try_find_brackets(editor, prev_position);
    };
    let screen_text = editor.content().iter().enumerate().skip(editor.cursor().at_line).take(editor.cursor().max_rows);
    let ranges = word.iter_encoded_word_ranges(screen_text, editor.lexer().encoding()).collect::<Vec<_>>();
    if ranges.is_empty() {
        return None;
    };
    Some(ranges)
}

fn try_find_brackets(editor: &Editor, position: CursorPosition) -> Option<Vec<EncodedWordRange>> {
    let ch_at_start = editor.content()[position.line].get_char(position.char)?;
    if let Some(opening) = get_opening(ch_at_start) {
        let mut counter = 0_usize;
        let mut char_idx = position.char;
        let first_line = &editor.content()[position.line][..position.char];
        for ch in first_line.chars().rev() {
            char_idx -= 1;
            if ch == ch_at_start {
                counter += 1;
            } else if ch == opening {
                if counter == 0 {
                    let start = (editor.lexer().encoding().str_len)(&editor.content()[position.line][..char_idx]);
                    let start_closing = (editor.lexer().encoding().str_len)(first_line);
                    return Some(vec![
                        EncodedWordRange::new_checked(position.line, start, start + 1)?,
                        EncodedWordRange::new_checked(position.line, start_closing, start_closing + 1)?,
                    ]);
                }
                counter -= 1;
            }
        }

        let limit = position.line.checked_sub(editor.cursor().at_line)?;
        for (line_idx, eline) in editor.content().iter().enumerate().skip(editor.cursor().at_line).take(limit).rev() {
            char_idx = eline.char_len();
            for ch in eline.chars().rev() {
                char_idx -= 1;
                if ch == ch_at_start {
                    counter += 1;
                } else if ch == opening {
                    if counter == 0 {
                        let start = (editor.lexer().encoding().str_len)(&editor.content()[line_idx][..char_idx]);
                        let start_closing = (editor.lexer().encoding().str_len)(first_line);
                        return Some(vec![
                            EncodedWordRange::new_checked(line_idx, start, start + 1)?,
                            EncodedWordRange::new_checked(position.line, start_closing, start_closing + 1)?,
                        ]);
                    }
                    counter -= 1;
                }
            }
        }
    } else if let Some(closing) = get_closing(ch_at_start) {
        let mut counter = 0_usize;
        let mut char_idx = position.char + 1;
        let first_line = &editor.content()[position.line][..position.char];
        for ch in editor.content()[position.line][char_idx..].chars() {
            if ch == ch_at_start {
                counter += 1;
            } else if ch == closing {
                let start = (editor.lexer().encoding().str_len)(first_line);
                let start_closing = (editor.lexer().encoding().str_len)(&editor.content()[position.line][..char_idx]);
                if counter == 0 {
                    return Some(vec![
                        EncodedWordRange::new_checked(position.line, start, start + 1)?,
                        EncodedWordRange::new_checked(position.line, start_closing, start_closing + 1)?,
                    ]);
                }
                counter -= 1;
            }
            char_idx += 1;
        }

        let skip = position.line + 1;
        let last_line = editor.cursor().at_line + editor.cursor().max_rows;
        let limit = last_line.checked_sub(skip)?;
        for (line_idx, eline) in editor.content().iter().enumerate().skip(skip).take(limit) {
            char_idx = 0;
            for ch in eline.chars() {
                if ch == ch_at_start {
                    counter += 1;
                } else if ch == closing {
                    if counter == 0 {
                        let start = (editor.lexer().encoding().str_len)(first_line);
                        let start_closing =
                            (editor.lexer().encoding().str_len)(&editor.content()[line_idx][..char_idx]);
                        return Some(vec![
                            EncodedWordRange::new_checked(position.line, start, start + 1)?,
                            EncodedWordRange::new_checked(line_idx, start_closing, start_closing + 1)?,
                        ]);
                    }
                    counter -= 1;
                }
                char_idx += 1;
            }
        }
    }

    None
}

fn get_closing(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        _ => None,
    }
}

fn get_opening(ch: char) -> Option<char> {
    match ch {
        '}' => Some('{'),
        ')' => Some('('),
        ']' => Some('['),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::try_find_brackets;
    use crate::{
        configs::FileType,
        cursor::{CursorPosition, EncodedWordRange},
        editor::syntax::tests::{mock_utf16_lexer, mock_utf8_lexer},
        editor::tests::mock_editor,
        editor_line::EditorLine,
    };

    #[test]
    fn test_eline_slicing() {
        let eline = EditorLine::from("🦀asd");
        assert_eq!("🦀", &eline[..1]);
        assert_eq!("🦀asd", &eline[0..]);
        assert_eq!("", &eline[..0]);
        let eline = EditorLine::from("1asd");
        assert_eq!("1", &eline[..1]);
        assert_eq!("1asd", &eline[0..]);
        assert_eq!("", &eline[..0]);
    }

    #[test]
    fn test_try_find_brackets() {
        let mut editor = mock_editor(vec!["data 🦀 {".to_owned(), "    text".to_owned(), " mm: 🦀 }".to_owned()]);

        let res = try_find_brackets(&editor, CursorPosition { line: 2, char: 7 });
        assert_eq!(res, Some(vec![EncodedWordRange::new(0, 7, 8), EncodedWordRange::new(2, 7, 8),]));

        editor.set_lexer(mock_utf16_lexer(FileType::Rust));
        let res = try_find_brackets(&editor, CursorPosition { line: 2, char: 7 });
        assert_eq!(res, Some(vec![EncodedWordRange::new(0, 8, 9), EncodedWordRange::new(2, 8, 9),]));

        editor.set_lexer(mock_utf8_lexer(FileType::Rust));
        let res = try_find_brackets(&editor, CursorPosition { line: 2, char: 7 });
        assert_eq!(res, Some(vec![EncodedWordRange::new(0, 10, 11), EncodedWordRange::new(2, 10, 11),]));
    }
}
