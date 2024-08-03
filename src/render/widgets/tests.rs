use crate::render::{
    backend::{color, Backend, BackendProtocol, Style}, layout::Line, widgets::Writable
};

use super::Text;

#[test]
fn test_basic_text() {
    let mut backend = Backend::init();
    let inner = String::from("asd🚀aa31ase字as");
    let as_text = Text::from(inner);
    assert_eq!(as_text.char_len(), 14);
    assert_eq!(as_text.width(), 16);
    assert_eq!(as_text.len(), 19);
    as_text.print(&mut backend);
    let data = backend.drain().into_iter().next().unwrap().1;
    assert_eq!(&data, "asd🚀aa31ase字as");
}

#[test]
fn test_text_truncate() {
    let mut backend = Backend::init();
    let inner = String::from("asd🚀aa31ase字as");
    let mut text = Text::from(inner);
    unsafe { text.print_truncated(4, &mut backend) };
    text.set_style(Some(Style::fg(color::blue())));
    unsafe { text.print_truncated_start(3, &mut backend) };
    text.set_style(None);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "asd".to_owned()),
            (Style::default(), "<<padding: 1>>".to_owned()),
            (Style::default(), "<<padding: 1>>".to_owned()),
            (Style::fg(color::blue()), "as".to_owned())
        ]
    );
}

#[test]
fn test_text_print_at() {
    let mut backend = Backend::init();
    let inner = String::from("asd🚀aa31ase字as");
    let text = Text::new(inner.clone(), Some(Style::fg(color::red())));
    let bigger_line = Line {row: 1, col: 1, width: 30};
    text.print_at(bigger_line, &mut backend);
    assert_eq!(backend.drain(), vec![
        (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
        (Style::fg(color::red()), inner),
        (Style::default(), "<<padding: 14>>".to_owned()),
    ]);
    let smaller_line = Line {row: 1, col: 1, width: 13};
    text.print_at(smaller_line, &mut backend);
    assert_eq!(vec![
        (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
        (Style::fg(color::red()), "asd🚀aa31ase".to_owned()),
        (Style::default(), "<<padding: 1>>".to_owned()),
    ], backend.drain());
}

#[test]
fn test_text_wrap() {}
