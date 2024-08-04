use crate::render::{
    backend::{color, Backend, BackendProtocol, Style},
    layout::{Line, Rect},
    widgets::Writable,
};

use super::{StyledLine, Text};

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
    let bigger_line = Line { row: 1, col: 1, width: 30 };
    text.print_at(bigger_line, &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::red()), inner),
            (Style::default(), "<<padding: 14>>".to_owned()),
        ]
    );
    let smaller_line = Line { row: 1, col: 1, width: 13 };
    text.print_at(smaller_line, &mut backend);
    assert_eq!(
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::red()), "asd🚀aa31ase".to_owned()),
            (Style::default(), "<<padding: 1>>".to_owned()),
        ],
        backend.drain()
    );
}

#[test]
fn test_text_wrap() {
    let mut backend = Backend::init();
    let rect = Rect::new(1, 1, 4, 10);
    let inner = String::from("asd🚀aa31ase字as");
    let text = Text::new(inner, Some(Style::fg(color::red())));
    text.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::red()), "asd".to_owned()),
            (Style::default(), "<<padding: 1>>".to_owned()),
            (Style::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (Style::fg(color::red()), "🚀aa".to_owned()),
            (Style::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (Style::fg(color::red()), "31as".to_owned()),
            (Style::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (Style::fg(color::red()), "e字a".to_owned()),
            (Style::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (Style::fg(color::red()), "s".to_owned()),
            (Style::default(), "<<padding: 3>>".to_owned())
        ]
    );

    let inner = String::from("asd123asd123asd123asd123");
    let text = Text::new(inner, Some(Style::fg(color::black())));
    text.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::black()), "asd1".to_owned()),
            (Style::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (Style::fg(color::black()), "23as".to_owned()),
            (Style::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (Style::fg(color::black()), "d123".to_owned()),
            (Style::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (Style::fg(color::black()), "asd1".to_owned()),
            (Style::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (Style::fg(color::black()), "23as".to_owned()),
            (Style::default(), "<<go to row: 6 col: 1>>".to_owned()),
            (Style::fg(color::black()), "d123".to_owned()),
        ]
    );
}

/// StyledLine
#[test]
fn test_line() {
    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(Style::fg(color::yellow()))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(")".to_string()),
        Text::from(":".to_string()),
    ]
    .into();
    assert_eq!(line.len(), 14);
    assert_eq!(line.width(), 14);
    assert_eq!(line.char_len(), 14);
}

#[test]
fn test_line_print() {
    let mut backend = Backend::init();
    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(Style::fg(color::yellow()))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::from("=".to_string()),
        Text::from(" ".to_string()),
        Text::from("\"🚀🚀\"".to_string()),
        Text::from(")".to_string()),
        Text::from(":".to_string()),
    ]
    .into();
    unsafe { line.print_truncated(17, &mut backend) }
    let mut expected = vec![
        (Style::fg(color::blue()), "def".to_owned()),
        (Style::default(), " ".to_owned()),
        (Style::fg(color::yellow()), "test".to_owned()),
        (Style::default(), "(".to_owned()),
        (Style::fg(color::blue()), "arg".to_owned()),
        (Style::default(), " ".to_owned()),
        (Style::default(), "=".to_owned()),
        (Style::default(), " ".to_owned()),
        (Style::default(), "\"".to_owned()),
        (Style::default(), "<<padding: 1>>".to_owned()),
    ];
    assert_eq!(backend.drain(), expected);
    unsafe { line.print_truncated_start(6, &mut backend) }
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<padding: 1>>".to_owned()),
            (Style::default(), "🚀\"".to_owned()),
            (Style::default(), ")".to_owned()),
            (Style::default(), ":".to_owned()),
        ]
    );

    let small_line = Line { row: 1, col: 1, width: 17 };
    expected.insert(0, (Style::default(), "<<go to row: 1 col: 1>>".to_owned()));
    line.print_at(small_line, &mut backend);
    assert_eq!(backend.drain(), expected);

    let bigger_line = Line { row: 1, col: 1, width: 40 };
    expected.pop();
    expected.pop();
    expected.extend([
        (Style::default(), "\"🚀🚀\"".to_owned()),
        (Style::default(), ")".to_owned()),
        (Style::default(), ":".to_owned()),
        (Style::default(), "<<padding: 17>>".to_owned()),
    ]);
    line.print_at(bigger_line, &mut backend);
    assert_eq!(backend.drain(), expected);
}

#[test]
fn test_line_wrap_complex() {
    let mut backend = Backend::init();
    let rect = Rect::new(1, 1, 7, 10);

    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(Style::fg(color::yellow()))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::from("=".to_string()),
        Text::from(" ".to_string()),
        Text::from("\"🚀🚀🚀🚀123\"".to_string()),
        Text::from(")".to_string()),
        Text::from(":".to_string()),
    ]
    .into();
    assert_eq!(line.char_len(), 26); // 26 chars
    assert_eq!(line.width(), 30); // 4 mojis x 2 char width
    assert_eq!(line.len(), 38); // 4 empjis x 4 bytes 26 - 4 = 22; 4 x 4 = 16; 22 + 16 = 38
    line.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::blue()), "def".to_owned()),   // 3
            (Style::default(), " ".to_owned()),             // 1
            (Style::fg(color::yellow()), "tes".to_owned()), // 3
            (Style::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (Style::fg(color::yellow()), "t".to_owned()), // 1
            (Style::default(), "(".to_owned()),           // 1
            (Style::fg(color::blue()), "arg".to_owned()), // 3
            (Style::default(), " ".to_owned()),           // 1
            (Style::default(), "=".to_owned()),           // 1
            (Style::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (Style::default(), " ".to_owned()),              // 1
            (Style::default(), "\"".to_owned()),             // 5
            (Style::default(), "🚀".to_owned()),             // 5
            (Style::default(), "🚀".to_owned()),             // 5
            (Style::default(), "<<padding: 1>>".to_owned()), // 1
            (Style::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (Style::default(), "🚀".to_owned()), // 2
            (Style::default(), "🚀".to_owned()), // 2
            (Style::default(), "1".to_owned()),  // 1
            (Style::default(), "2".to_owned()),  // 1
            (Style::default(), "3".to_owned()),  // 1
            (Style::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (Style::default(), "\"".to_owned()),             // 1
            (Style::default(), ")".to_owned()),              // 1
            (Style::default(), ":".to_owned()),              // 1
            (Style::default(), "<<padding: 4>>".to_owned()), // 4
        ]
    );
}

#[test]
fn test_line_wrap_simple() {
    let mut backend = Backend::init();
    let rect = Rect::new(1, 1, 7, 10);

    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(Style::fg(color::yellow()))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(Style::fg(color::blue()))),
        Text::from(" ".to_string()),
        Text::from("=".to_string()),
        Text::from(" ".to_string()),
        Text::from("\"really long text goest here - needs >14\"".to_string()),
        Text::from(")".to_string()),
        Text::from(":".to_string()),
    ]
    .into();
    assert_eq!(line.char_len(), 58);
    assert_eq!(line.width(), 58);
    assert_eq!(line.len(), 58);
    line.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (Style::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (Style::fg(color::blue()), "def".to_owned()),   // 3
            (Style::default(), " ".to_owned()),             // 1
            (Style::fg(color::yellow()), "tes".to_owned()), // 3
            (Style::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (Style::fg(color::yellow()), "t".to_owned()), // 1
            (Style::default(), "(".to_owned()),           // 1
            (Style::fg(color::blue()), "arg".to_owned()), // 3
            (Style::default(), " ".to_owned()),           // 1
            (Style::default(), "=".to_owned()),           // 1
            (Style::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (Style::default(), " ".to_owned()),       // 1
            (Style::default(), "\"reall".to_owned()), // 6
            (Style::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (Style::default(), "y long ".to_owned()), // 7
            (Style::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (Style::default(), "text go".to_owned()), // 7
            (Style::default(), "<<go to row: 6 col: 1>>".to_owned()),
            (Style::default(), "est her".to_owned()), // 7
            (Style::default(), "<<go to row: 7 col: 1>>".to_owned()),
            (Style::default(), "e - nee".to_owned()), // 7
            (Style::default(), "<<go to row: 8 col: 1>>".to_owned()),
            (Style::default(), "ds >14\"".to_owned()), // 7
            (Style::default(), "<<go to row: 9 col: 1>>".to_owned()),
            (Style::default(), ")".to_owned()),              // 1
            (Style::default(), ":".to_owned()),              // 1
            (Style::default(), "<<padding: 5>>".to_owned()), // 5
        ]
    );
}
