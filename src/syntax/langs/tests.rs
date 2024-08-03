use crate::{
    configs::FileType,
    render::{
        backend::{Backend, BackendProtocol},
        layout::Rect,
        widgets::{StyledLine, Text, Writable},
        UTF8Safe,
    },
    syntax::theme::Theme,
};

use super::Lang;

pub fn create_text() -> [String; 16] {
    [
        String::from("You can create a String from a literal string with [String::from]:"),
        String::from(""),
        String::from("let hello = String::from(\"Hello, world!\");"),
        String::from(
            "You can append a char to a String with the [push] method, and append a [&str] with the [push_str] method:",
        ),
        String::from(""),
        String::from("let mut hello = String::from(\"Hello, \");"),
        String::from("hello.push('w');"),
        String::from("hello.push_str(\"orld!\");"),
        String::from(
            "If you have a vector of UTF-8 bytes, you can create a String from it with the [from_utf8] method:",
        ),
        String::from("// some bytes, in a vector"),
        String::from("let sparkle_heart = vec![240, 159, 146, 150];"),
        String::from(""),
        String::from("// We know these bytes are valid, so we'll use `unwrap()`."),
        String::from("et sparkle_heart = String::from_utf8(sparkle_heart).unwrap();"),
        String::from(""),
        String::from("assert_eq!(\"💖\", sparkle_heart);"),
    ]
}

#[test]
fn test_stylize() {
    let mut backend = Backend::init();
    let rect = Rect::new(10, 1, 60, 6);
    let mut lines = rect.into_iter();
    let theme = Theme::default();
    let lang = Lang::from(FileType::Rust);
    let inputs = create_text();
    let styled_lines = inputs.iter().map(|text_line| lang.stylize(text_line.to_owned(), &theme)).enumerate();
    for (idx, sline) in styled_lines {
        assert_eq!(sline.len(), inputs[idx].len());
        assert_eq!(sline.char_len(), inputs[idx].char_len());
        assert_eq!(sline.width(), inputs[idx].width());
        sline.wrap(&mut lines, &mut backend);
    }
    panic!("{:?}", backend.drain());
}
