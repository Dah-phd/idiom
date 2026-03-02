use super::TuiCodec;
use crate::{
    editor::tests::mock_editor,
    editor_line::RenderStatus,
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{layout::Rect, Backend};

pub fn expect_select(
    mut start_char: usize,
    end_char: usize,
    select: Color,
    accent: ContentStyle,
    rendered: &[(ContentStyle, String)],
) {
    let mut count_to_end = end_char - start_char;
    let tokens = rendered
        .iter()
        .skip_while(|(.., t)| t != "<<clear EOL>>")
        .take_while(|(.., t)| !t.starts_with("<<go to row"))
        .filter(|(c, t)| {
            let is_ui = *c == accent;
            let is_control = t.starts_with("<<") && t.ends_with(">>");
            !is_ui && !is_control
        });

    for (style, text) in tokens {
        if start_char != 0 {
            assert_eq!(style.background_color, None);
            start_char -= text.chars().count();
        } else if count_to_end != 0 {
            assert_eq!(style.background_color, Some(select));
            count_to_end -= text.chars().count();
        } else {
            assert_eq!(style.background_color, None)
        }
    }
}

pub fn expect_cursor(mut char_idx: usize, skip_until: &str, rendered: &[(ContentStyle, String)]) {
    let mut skip = true;
    for (style, text) in rendered.iter() {
        if skip {
            skip = text != skip_until;
            continue;
        }

        if char_idx != 0 {
            char_idx -= text.chars().count();
            continue;
        }
        assert_eq!(*style, ContentStyle::reversed());
        return;
    }
    panic!("Cursor not found!\n{rendered:?}")
}

pub fn count_to_cursor(accent_style: ContentStyle, rendered: &[(ContentStyle, String)]) -> usize {
    let mut cursor = 0;
    for (style, text) in rendered.iter().skip_while(|(.., t)| t != "<<clear EOL>>") {
        if accent_style == *style || (text.starts_with("<<") && text.ends_with(">>")) {
            continue;
        }
        if *style == ContentStyle::reversed() {
            return cursor;
        }
        cursor += text.chars().count();
    }
    panic!("Unable to find cursor!")
}

pub fn parse_simple_line(rendered: &mut Vec<(ContentStyle, String)>) -> (Option<usize>, Vec<String>) {
    let mut line_idx = None;
    for (idx, (_, txt)) in rendered.iter().enumerate() {
        if !txt.starts_with("<<go to row") {
            line_idx = txt.trim().parse().ok();
            rendered.drain(..idx + 2);
            break;
        }
    }
    for (idx, (_, t)) in rendered.iter().enumerate() {
        if t.starts_with("<<go to row") {
            return (line_idx, rendered.drain(..idx).map(|(_, t)| t).collect());
        }
    }
    (line_idx, rendered.drain(..).map(|(_, t)| t).collect())
}

pub fn parse_complex_line(rendered: &mut Vec<(ContentStyle, String)>) -> (Option<usize>, Vec<String>) {
    let (line_idx, raw_data) = parse_simple_line(rendered);
    let mut parsed = vec![];
    let mut current = String::new();
    let mut first = true;
    for part in raw_data {
        if part.starts_with("<<") {
            if first {
                continue;
            }
            parsed.push(std::mem::take(&mut current));
        } else {
            current.push_str(&part);
        }
        first = false;
    }
    if !current.is_empty() {
        parsed.push(current);
    }
    (line_idx, parsed)
}

#[test]
fn test_has_render_cache() {
    let mut editor = mock_editor(vec![
        "test".into(),
        String::new(),
        "more test text".into(),
        "4 lines are enough".into(),
    ]);
    editor.cursor.text_width = 80;
    editor.cursor.max_rows = 10;
    let mut gs = GlobalState::new(Rect::new(0, 0, 80, 10), CrossTerm::init());
    gs.force_area_calc();

    assert!(!editor.has_render_cache());
    editor.render(&mut gs);
    assert!(editor.has_render_cache());
    editor.content[1].insert_str(0, "fill");
    assert!(!editor.has_render_cache());
    editor.render(&mut gs);
    assert!(editor.has_render_cache());
    editor.cursor.set_position((1, 2).into());
    editor.cursor.select_word(&editor.content);
    assert!(editor.cursor.select_get().is_some());
    assert!(editor.has_render_cache());
}

#[test]
fn test_is_full_render_or_invalidate_line() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 80, 8), CrossTerm::init());
    gs.force_area_calc();

    let mut editor = mock_editor(vec![
        "use some_package".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
    ]);

    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    assert_eq!(editor.cursor.max_rows, 6);

    editor.render(&mut gs);
    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));

    assert_eq!(editor.content.len(), 19);
    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));

    let five = editor.content.remove(5);
    assert_eq!(editor.content.len(), 18);
    editor.render(&mut gs);
    assert!(!editor.content[5].cached.is_none());

    editor.content.insert(5, five);
    assert!(!editor.content[6].cached.is_none());
    let sixth = editor.content[6].to_string();
    editor.render(&mut gs);

    editor.content.remove(5);

    let mut expect_rend_stat = RenderStatus::None;
    expect_rend_stat.line(6, None);

    assert_eq!(editor.content[5].cached, expect_rend_stat);
    assert_eq!(editor.content[5].as_str(), sixth.as_str());

    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));
    assert!(editor.content[5].cached.is_none());
}

#[test]
fn test_is_full_render_or_invalidate_lines() {
    let mut gs = GlobalState::new(Rect::new(0, 0, 80, 8), CrossTerm::init());
    gs.force_area_calc();

    let mut editor = mock_editor(vec![
        "use some_package".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
        String::new(),
        "fn main() {".into(),
        "    let data = \"adadwas\";".into(),
        "    some_pacakge::do_something(data);".into(),
        "    println!(\"{:?}\", data);".into(),
        "}".into(),
    ]);

    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);
    assert_eq!(editor.cursor.max_rows, 6);

    editor.render(&mut gs);
    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));

    assert_eq!(editor.content.len(), 19);
    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));

    let four = editor.content.remove(4);
    let five = editor.content.remove(4);
    assert_eq!(editor.content.len(), 17);
    editor.render(&mut gs);
    assert!(!editor.content[4].cached.is_none() && !editor.content[5].cached.is_none());

    editor.content.insert(4, five);
    editor.content.insert(4, four);
    assert!(!editor.content[6].cached.is_none() && !editor.content[7].cached.is_none());
    let sixth = editor.content[6].to_string();
    let seventh = editor.content[7].to_string();
    editor.render(&mut gs);

    editor.content.remove(4);
    editor.content.remove(4);

    // ensure stuct is correct
    let mut expect_rend_stat = RenderStatus::None;

    expect_rend_stat.line(5, None);
    assert_eq!(editor.content[4].cached, expect_rend_stat);
    assert_eq!(editor.content[4].as_str(), sixth.as_str());

    expect_rend_stat.line(6, None);
    assert_eq!(editor.content[5].cached, expect_rend_stat);
    assert_eq!(editor.content[5].as_str(), seventh.as_str());

    assert!(!TuiCodec::is_full_render_or_invalidate_lines(&mut editor));
    assert!(editor.content[4].cached.is_none());
    assert!(editor.content[5].cached.is_none());
}
