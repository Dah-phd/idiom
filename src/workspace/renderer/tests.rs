use crate::render::backend::StyleExt;
use crossterm::style::{Color, ContentStyle};

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

pub fn expect_cursor(mut char_idx: usize, rendered: &[(ContentStyle, String)]) {
    let mut skip = true;
    for (style, text) in rendered.iter() {
        if skip {
            skip = text != "<<clear EOL>>";
            continue;
        }

        if char_idx != 0 {
            char_idx -= text.chars().count();
            continue;
        }
        assert_eq!(*style, ContentStyle::reversed());
        return;
    }
    panic!("Cursor not found!")
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
