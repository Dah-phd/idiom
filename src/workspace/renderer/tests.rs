use crossterm::style::ContentStyle;

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
