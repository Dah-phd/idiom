pub enum TextType {
    Plain,
    MarkDown,
}

impl TextType {}

pub enum MDLineBreak {
    Spaces,
    NLines,
    Tag,
    TagNL, // with new line at the end
    Backslash,
    EoF,
}

impl MDLineBreak {
    pub fn split_lines(text: String) -> Vec<(String, Self)> {
        let mut buffer = vec![];
        let mut line_text = String::new();
        let mut str_iter = text.chars().peekable();
        for ch in str_iter.by_ref() {
            line_text.push(ch);
            if line_text.ends_with("  \n") {
                buffer.push((std::mem::take(&mut line_text), MDLineBreak::Spaces));
            } else if line_text.ends_with("\n\n") {
                buffer.push((std::mem::take(&mut line_text), MDLineBreak::NLines));
            } else if line_text.ends_with("<br/ >\n") {
                buffer.push((std::mem::take(&mut line_text), MDLineBreak::TagNL));
            } else if line_text.ends_with("\\\n") {
                buffer.push((std::mem::take(&mut line_text), MDLineBreak::Backslash));
            };
        }
        buffer.push((line_text, MDLineBreak::EoF));
        buffer
    }
}
