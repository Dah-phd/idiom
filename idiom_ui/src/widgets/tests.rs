use crate::{
    backend::{Backend, CrossTerm, StyleExt},
    layout::{Line, Rect},
    widgets::Writable,
};

use super::{StyledLine, Text};

use crossterm::style::{Color, ContentStyle};

#[test]
fn test_basic_text() {
    let mut backend = CrossTerm::init();
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
    let mut backend = CrossTerm::init();
    let inner = String::from("asd🚀aa31ase字as");
    let mut text = Text::from(inner);
    unsafe { text.print_truncated(4, &mut backend) };
    text.set_style(Some(ContentStyle::fg(Color::Blue)));
    unsafe { text.print_truncated_start(3, &mut backend) };
    text.set_style(None);
    assert_eq!(
        backend.drain(),
        vec![
            (ContentStyle::default(), "asd".to_owned()),
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
            (ContentStyle::fg(Color::Blue), "as".to_owned())
        ]
    );
}

#[test]
fn test_text_print_at() {
    let mut backend = CrossTerm::init();
    let inner = String::from("asd🚀aa31ase字as");
    let text = Text::new(inner.clone(), Some(ContentStyle::fg(Color::Red)));
    let bigger_line = Line { row: 1, col: 1, width: 30 };
    text.print_at(bigger_line, &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), inner),
            (ContentStyle::default(), "<<padding: 14>>".to_owned()),
        ]
    );
    let smaller_line = Line { row: 1, col: 1, width: 13 };
    text.print_at(smaller_line, &mut backend);
    assert_eq!(
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "asd🚀aa31ase".to_owned()),
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
        ],
        backend.drain()
    );
}

#[test]
fn test_text_wrap() {
    let mut backend = CrossTerm::init();
    let rect = Rect::new(1, 1, 4, 10);
    let inner = String::from("asd🚀aa31ase字as");
    let text = Text::new(inner, Some(ContentStyle::fg(Color::Red)));
    text.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "asd".to_owned()),
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "🚀aa".to_owned()),
            (ContentStyle::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "31as".to_owned()),
            (ContentStyle::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "e字a".to_owned()),
            (ContentStyle::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Red), "s".to_owned()),
            (ContentStyle::default(), "<<padding: 3>>".to_owned())
        ]
    );

    let inner = String::from("asd123asd123asd123asd123");
    let text = Text::new(inner, Some(ContentStyle::fg(Color::Black)));
    text.wrap(&mut rect.into_iter(), &mut backend);
    assert_eq!(
        backend.drain(),
        vec![
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "asd1".to_owned()),
            (ContentStyle::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "23as".to_owned()),
            (ContentStyle::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "d123".to_owned()),
            (ContentStyle::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "asd1".to_owned()),
            (ContentStyle::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "23as".to_owned()),
            (ContentStyle::default(), "<<go to row: 6 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Black), "d123".to_owned()),
        ]
    );
}

/// StyledLine
#[test]
fn test_line() {
    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(ContentStyle::fg(Color::Blue))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(ContentStyle::fg(Color::Yellow))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(ContentStyle::fg(Color::Blue))),
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
    let mut backend = CrossTerm::init();
    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(ContentStyle::fg(Color::Blue))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(ContentStyle::fg(Color::Yellow))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(ContentStyle::fg(Color::Blue))),
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
        (ContentStyle::fg(Color::Blue), "def".to_owned()),
        (ContentStyle::default(), " ".to_owned()),
        (ContentStyle::fg(Color::Yellow), "test".to_owned()),
        (ContentStyle::default(), "(".to_owned()),
        (ContentStyle::fg(Color::Blue), "arg".to_owned()),
        (ContentStyle::default(), " ".to_owned()),
        (ContentStyle::default(), "=".to_owned()),
        (ContentStyle::default(), " ".to_owned()),
        (ContentStyle::default(), "\"".to_owned()),
        (ContentStyle::default(), "<<padding: 1>>".to_owned()),
    ];
    assert_eq!(backend.drain(), expected);
    unsafe { line.print_truncated_start(6, &mut backend) }
    assert_eq!(
        backend.drain(),
        vec![
            (ContentStyle::default(), "<<padding: 1>>".to_owned()),
            (ContentStyle::default(), "🚀\"".to_owned()),
            (ContentStyle::default(), ")".to_owned()),
            (ContentStyle::default(), ":".to_owned()),
        ]
    );

    let small_line = Line { row: 1, col: 1, width: 17 };
    expected.insert(0, (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()));
    line.print_at(small_line, &mut backend);
    assert_eq!(backend.drain(), expected);

    let bigger_line = Line { row: 1, col: 1, width: 40 };
    expected.pop();
    expected.pop();
    expected.extend([
        (ContentStyle::default(), "\"🚀🚀\"".to_owned()),
        (ContentStyle::default(), ")".to_owned()),
        (ContentStyle::default(), ":".to_owned()),
        (ContentStyle::default(), "<<padding: 17>>".to_owned()),
    ]);
    line.print_at(bigger_line, &mut backend);
    assert_eq!(backend.drain(), expected);
}

#[test]
fn test_line_wrap_complex() {
    let mut backend = CrossTerm::init();
    let rect = Rect::new(1, 1, 7, 10);

    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(ContentStyle::fg(Color::Blue))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(ContentStyle::fg(Color::Yellow))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(ContentStyle::fg(Color::Blue))),
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
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Blue), "def".to_owned()),   // 3
            (ContentStyle::default(), " ".to_owned()),           // 1
            (ContentStyle::fg(Color::Yellow), "tes".to_owned()), // 3
            (ContentStyle::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Yellow), "t".to_owned()), // 1
            (ContentStyle::default(), "(".to_owned()),         // 1
            (ContentStyle::fg(Color::Blue), "arg".to_owned()), // 3
            (ContentStyle::default(), " ".to_owned()),         // 1
            (ContentStyle::default(), "=".to_owned()),         // 1
            (ContentStyle::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (ContentStyle::default(), " ".to_owned()),              // 1
            (ContentStyle::default(), "\"".to_owned()),             // 5
            (ContentStyle::default(), "🚀".to_owned()),             // 5
            (ContentStyle::default(), "🚀".to_owned()),             // 5
            (ContentStyle::default(), "<<padding: 1>>".to_owned()), // 1
            (ContentStyle::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (ContentStyle::default(), "🚀".to_owned()), // 2
            (ContentStyle::default(), "🚀".to_owned()), // 2
            (ContentStyle::default(), "1".to_owned()),  // 1
            (ContentStyle::default(), "2".to_owned()),  // 1
            (ContentStyle::default(), "3".to_owned()),  // 1
            (ContentStyle::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (ContentStyle::default(), "\"".to_owned()),             // 1
            (ContentStyle::default(), ")".to_owned()),              // 1
            (ContentStyle::default(), ":".to_owned()),              // 1
            (ContentStyle::default(), "<<padding: 4>>".to_owned()), // 4
        ]
    );
}

#[test]
fn test_line_wrap_simple() {
    let mut backend = CrossTerm::init();
    let rect = Rect::new(1, 1, 7, 10);

    let line: StyledLine = vec![
        Text::new("def".to_owned(), Some(ContentStyle::fg(Color::Blue))),
        Text::from(" ".to_string()),
        Text::new("test".to_owned(), Some(ContentStyle::fg(Color::Yellow))),
        Text::from("(".to_string()),
        Text::new("arg".to_owned(), Some(ContentStyle::fg(Color::Blue))),
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
            (ContentStyle::default(), "<<go to row: 1 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Blue), "def".to_owned()),   // 3
            (ContentStyle::default(), " ".to_owned()),           // 1
            (ContentStyle::fg(Color::Yellow), "tes".to_owned()), // 3
            (ContentStyle::default(), "<<go to row: 2 col: 1>>".to_owned()),
            (ContentStyle::fg(Color::Yellow), "t".to_owned()), // 1
            (ContentStyle::default(), "(".to_owned()),         // 1
            (ContentStyle::fg(Color::Blue), "arg".to_owned()), // 3
            (ContentStyle::default(), " ".to_owned()),         // 1
            (ContentStyle::default(), "=".to_owned()),         // 1
            (ContentStyle::default(), "<<go to row: 3 col: 1>>".to_owned()),
            (ContentStyle::default(), " ".to_owned()),       // 1
            (ContentStyle::default(), "\"reall".to_owned()), // 6
            (ContentStyle::default(), "<<go to row: 4 col: 1>>".to_owned()),
            (ContentStyle::default(), "y long ".to_owned()), // 7
            (ContentStyle::default(), "<<go to row: 5 col: 1>>".to_owned()),
            (ContentStyle::default(), "text go".to_owned()), // 7
            (ContentStyle::default(), "<<go to row: 6 col: 1>>".to_owned()),
            (ContentStyle::default(), "est her".to_owned()), // 7
            (ContentStyle::default(), "<<go to row: 7 col: 1>>".to_owned()),
            (ContentStyle::default(), "e - nee".to_owned()), // 7
            (ContentStyle::default(), "<<go to row: 8 col: 1>>".to_owned()),
            (ContentStyle::default(), "ds >14\"".to_owned()), // 7
            (ContentStyle::default(), "<<go to row: 9 col: 1>>".to_owned()),
            (ContentStyle::default(), ")".to_owned()),              // 1
            (ContentStyle::default(), ":".to_owned()),              // 1
            (ContentStyle::default(), "<<padding: 5>>".to_owned()), // 5
        ]
    );
}
