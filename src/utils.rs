use crate::components::editor::Offset;

pub fn trim_start_inplace(line: &mut String) -> Offset {
    if let Some(idx) = line.find(|c: char| !c.is_whitespace()) {
        line.replace_range(..idx, "");
        return Offset::Neg(idx + 1);
    };
    Offset::Pos(0)
}

pub fn get_closing_char(ch: char) -> Option<char> {
    match ch {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}
