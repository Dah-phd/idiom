use crate::{
    embeded_term::EditorTerminal,
    ext_tui::StyleExt,
    global_state::GlobalState,
    syntax::{tokens::TokenLine, Lexer},
    tree::Tree,
    workspace::{
        cursor::{EncodedWordRange, PositionedWord},
        Editor, Workspace,
    },
};
use crossterm::{
    self,
    event::{poll, read, Event},
    style::{Attribute, Attributes, ContentStyle},
};
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
    let position = editor.controls.get_base_cursor_position().unwrap_or(editor.cursor.get_position());
    let Some(word) = PositionedWord::find_at(&editor.content, position) else {
        return Ok(());
    };
    let screen_text = editor.content.iter().enumerate().skip(editor.cursor.at_line).take(editor.cursor.max_rows);
    let ranges = word.iter_encoded_word_ranges(screen_text, editor.lexer.encoding()).collect::<Vec<_>>();
    if ranges.is_empty() {
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
            let screen_text =
                editor.content.iter().enumerate().skip(editor.cursor.at_line).take(editor.cursor.max_rows);
            let new_ranges = word.iter_encoded_word_ranges(screen_text, editor.lexer.encoding()).collect::<Vec<_>>();
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
        let line = &mut editor.content[range_line];
        if stored_tokens.iter().any(|(line, _)| line == &range_line) {
            line.tokens_mut_unchecked().set_encoded_word_checked(word, style);
        } else {
            let mut new_tokens = line.tokens().clone();
            new_tokens.set_encoded_word_checked(word, style);
            stored_tokens.push((range_line, std::mem::replace(line.tokens_mut(), new_tokens)));
        }
    }

    editor.render(gs);

    for (idx, tokens) in stored_tokens {
        *editor.content[idx].tokens_mut_unchecked() = tokens;
    }
}

fn clear_marked_cache(editor: &mut Editor, ranges: Vec<EncodedWordRange>) {
    for range in ranges {
        editor.content[range.line()].cached.reset();
    }
}
