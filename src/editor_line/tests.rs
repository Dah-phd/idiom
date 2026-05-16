pub use crate::editor_line::parser::{
    CARRIAGE_NLINE, FILE_END, FROM_FEED, LineParser, MSDOS_NLINE, POSIX_NLINE, RECORD_END, RISCOS_NLINE, VERTICAL_TAB,
};
use crate::{
    configs::IndentConfigs,
    editor::{syntax::Encoding, tests::mock_editor_from_elines},
    editor_line::EditorLine,
    ext_tui::{CrossTerm, StyleExt},
    global_state::GlobalState,
    popups::generic_popup::BasicConfirmPopup,
};
use crossterm::style::{Color, ContentStyle};
use idiom_tui::{Backend, layout::Rect};
use logos::Logos;

#[test]
fn test_get() {
    let line = EditorLine::from("bytb 🚀 asd 🚀");
    assert_eq!(line.get(1, 2), Some("y"));
    assert_eq!(line.get(2, 3), Some("t"));
    assert_eq!(line.get(5, 6), Some("🚀"));
    assert_eq!(line.get(10, 11), Some(" "));
    assert_eq!(line.get(11, 12), Some("🚀"));
    assert_eq!(line.get(11, 11), Some(""));
    assert_eq!(line.get(100, 200), None);
}

#[test]
fn trim_counted() {
    let line = EditorLine::from("   // coment");
    assert_eq!(line.trim_start_counted(), (3, "// coment"));
    let line = EditorLine::from("  🚀 //coment");
    assert_eq!(line.trim_start_counted(), (2, "🚀 //coment"));
}

#[test]
fn test_insert() {
    let mut line = EditorLine::new_posix("text".to_owned());
    assert!(line.char_len() == 4);
    let encoding = Encoding::utf32();
    line.insert_simple(2, 'e', &encoding);
    assert!(line.is_simple());
    line.insert_simple(2, '🚀', &encoding);
    assert!(line.char_len() == 6);
    assert!(!line.is_simple());
    line.insert_simple(3, 'x', &encoding);
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "te🚀xext");
}

#[test]
fn test_insert_cyrillic() {
    let mut line = EditorLine::new_posix("text".to_owned());
    assert!(line.char_len() == 4);
    let encoding = Encoding::utf32();
    line.insert_simple(2, 'g', &encoding);
    assert!(line.is_simple());
    line.insert_simple(2, 'з', &encoding);
    assert_eq!(line.char_len(), 6);
    assert!(!line.is_simple());
    line.insert_simple(3, 'й', &encoding);
    assert_eq!(line.char_len(), 7);
    assert_eq!(&line.to_string(), "teзйgxt");
}

#[test]
fn test_insert_simple_check_corect() {
    let mut line = EditorLine::new_posix("text".to_owned());
    let mut encoding = Encoding::utf32();
    encoding.mock_panic_on_insert_char_with_idx();
    line.insert_simple(2, 'x', &encoding);
    assert_eq!(line.as_str(), "texxt");
    line.insert_simple(2, 'г', &encoding);
    assert_eq!(line.as_str(), "teгxxt");
}

#[should_panic]
#[test]
fn test_insert_simple_check_corect_panic() {
    let mut line = EditorLine::new_posix("teгxt".to_owned());
    let mut encoding = Encoding::utf32();
    encoding.mock_panic_on_insert_char_with_idx();
    line.insert_simple(2, 'x', &encoding);
}

#[test]
fn test_insert_str() {
    let mut line = EditorLine::new_posix("text".to_owned());
    line.insert_str(0, "text");
    assert!(line.is_simple());
    assert!(line.char_len() == 8);
    line.insert_str(1, "rocket🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "trocket🚀exttext");
    assert!(line.char_len() < line.to_string().len());
}

#[test]
fn test_push() {
    let encoding = Encoding::utf32();
    let mut line = EditorLine::new_posix("text".to_owned());
    line.push_simple('1', &encoding);
    assert!(line.is_simple());
    assert!(line.char_len() == 5);
    line.push_simple('🚀', &encoding);
    assert!(!line.is_simple());
    assert!(line.to_string().len() == 9);
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text1🚀");
}

#[test]
fn test_push_with_cyrillic() {
    let encoding = Encoding::utf32();
    let mut line = EditorLine::new_posix("text".to_owned());
    line.push_simple('i', &encoding);
    assert!(line.is_simple());
    assert!(line.char_len() == 5);
    line.push_simple('г', &encoding);
    line.push_simple('г', &encoding);
    assert!(!line.is_simple());
    assert_eq!(line.to_string().len(), 9);
    assert_eq!(line.char_len(), 7);
    assert_eq!(&line.to_string(), "textiгг");
}

#[test]
fn test_push_str() {
    let mut line = EditorLine::new_posix(String::new());
    assert!(line.is_simple());
    assert!(line.char_len() == 0);
    line.push_str("text");
    assert!(line.is_simple());
    assert!(line.char_len() == 4);
    line.push_str("text🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "texttext🚀");
    assert!(line.char_len() == 9);
    assert!(line.to_string().len() == 12);
}

#[test]
fn test_replace_range() {
    let mut line = EditorLine::new_posix(String::from("🚀123"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_range(0..2, "text");
    assert!(line.is_simple());
    assert!(&line.to_string() == "text23");
    assert!(line.char_len() == 6);
    line.replace_range(3..6, "🚀🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "tex🚀🚀");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_till() {
    let mut line = EditorLine::new_posix(String::from("🚀123"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_till(3, "text");
    assert!(line.is_simple());
    assert!(&line.to_string() == "text3");
    assert!(line.char_len() == 5);
    line.replace_till(2, "🚀🚀");
    assert!(!line.is_simple());
    assert!(&line.to_string() == "🚀🚀xt3");
    assert!(line.char_len() == 5);
}

#[test]
fn test_replace_from() {
    let mut line = EditorLine::new_posix(String::from("123🚀"));
    assert!(!line.is_simple());
    assert!(line.char_len() == 4);
    line.replace_from(3, "text");
    assert!(line.is_simple());
    assert!(line.char_len() == 7);
    assert!(&line.to_string() == "123text");
    line.replace_from(3, "🚀🚀");
    assert!(!line.is_simple());
    assert!(line.char_len() == 5);
    assert!(&line.to_string() == "123🚀🚀");
}

#[test]
fn test_remove() {
    let mut line = EditorLine::new_posix("text🚀123".to_owned());
    let encoding = Encoding::utf32();
    assert!(!line.is_simple());
    assert!(line.char_len() == 8);
    assert!('1' == line.remove(5, &encoding));
    assert!(line.char_len() == 7);
    assert!(!line.is_simple());
    assert!('🚀' == line.remove(4, &encoding));
    assert!(line.is_simple());
    assert!(line.char_len() == 6);
    assert!(&line.to_string() == "text23");
}

#[test]
fn test_utf8_idx_at() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf8_idx_at(4));
    assert_eq!(2, line.unsafe_utf8_idx_at(2));
    assert_eq!(8, line.unsafe_utf8_idx_at(5));
    assert_eq!(10, line.unsafe_utf8_idx_at(7));
    assert_eq!(15, line.unsafe_utf8_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf8_idx_at_panic() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    line.unsafe_utf8_idx_at(10);
}

#[test]
fn test_utf16_idx_at() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    assert_eq!(4, line.unsafe_utf16_idx_at(4));
    assert_eq!(2, line.unsafe_utf16_idx_at(2));
    assert_eq!(6, line.unsafe_utf16_idx_at(5));
    assert_eq!(8, line.unsafe_utf16_idx_at(7));
    assert_eq!(11, line.unsafe_utf16_idx_at(9));
}

#[test]
#[should_panic]
fn test_utf16_idx_at_panic() {
    let line = EditorLine::new_posix("text🚀123🚀".to_owned());
    line.unsafe_utf16_idx_at(10);
}

#[test]
fn test_split_off() {
    let mut line = EditorLine::new_posix("text🚀123🚀".to_owned());
    line = line.split_off(2);
    assert_eq!(line.to_string(), "xt🚀123🚀");
    assert_eq!(line.char_len(), 7);
    assert_eq!(line.len(), 13);
    let new = line.split_off(4);
    assert_eq!(new.char_len(), 3);
    assert_eq!(new.len(), 6);
    assert_eq!(new.unwrap(), "23🚀");
}

#[test]
fn test_split_off_ascii() {
    let mut line = EditorLine::new_posix("texttext".to_owned());
    let remaining = line.split_off(4);
    assert_eq!(remaining.char_len(), 4);
    assert_eq!(remaining.len(), 4);
    assert_eq!(remaining.to_string(), "text");
    assert_eq!(line.char_len(), 4);
    assert_eq!(line.len(), 4);
    assert_eq!(line.to_string(), "text");
    assert_eq!(line.to_string(), "text");
}

#[test]
fn test_parse() {
    let text = "a💀w\ndawda\radaw\r\nwas\n_💀_\n\r+++\nada\u{001E}ae\u{001C}ddd\u{000C}awes\u{000B}aw";
    let content = LineParser::split_lines(text).into_editor_lines();
    let expect = [
        EditorLine::new("a💀w".into(), POSIX_NLINE),
        EditorLine::new("dawda".into(), CARRIAGE_NLINE),
        EditorLine::new("adaw".into(), MSDOS_NLINE),
        EditorLine::new("was".into(), POSIX_NLINE),
        EditorLine::new("_💀_".into(), RISCOS_NLINE),
        EditorLine::new("+++".into(), POSIX_NLINE),
        EditorLine::new("ada".into(), RECORD_END),
        EditorLine::new("ae".into(), FILE_END),
        EditorLine::new("ddd".into(), FROM_FEED),
        EditorLine::new("awes".into(), VERTICAL_TAB),
        EditorLine::new("aw".into(), POSIX_NLINE),
    ];
    assert_eq!(content.len(), expect.len());
    for (real, expect) in content.iter().zip(expect.iter()) {
        assert_eq!(real.as_str(), expect.as_str());
        assert_eq!(real.end(), expect.end());
    }
}

#[test]
fn parser_stable() {
    let data = "asd\n\tdaw\n\radwda\r\nadwd\rada\u{001E}ae\u{001C}ddd\u{000C}awes\u{000B}awdwe";
    let tokens = LineParser::lexer(data);
    assert_eq!(
        tokens.collect::<Vec<_>>(),
        [
            Ok(LineParser::POSIX_NEWLINE),
            Ok(LineParser::TAB),
            Ok(LineParser::RISCOS_NEWLINE),
            Ok(LineParser::MSDOS_NEWLINE),
            Ok(LineParser::CARRIAGE_NEWLINE),
            Ok(LineParser::RECORD_END),
            Ok(LineParser::FILE_END),
            Ok(LineParser::FROM_FEED),
            Ok(LineParser::VERTICAL_TAB),
        ],
    );
}

#[test]
fn test_render_line_ends() {
    let data = "aaa\rbbb\r\nccc\n\rddd\u{001E}ae\u{001C}ddd\u{000C}awes\u{000B}awdwe";
    let parsed = LineParser::split_lines(&data);
    let mut editor = mock_editor_from_elines(parsed.into_editor_lines());
    let mut gs = GlobalState::new(Rect::new(0, 0, 40, 5), CrossTerm::init());
    gs.force_area_calc();
    editor.resize(gs.editor_area().width, gs.editor_area().height as usize);

    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "<<reset style>>",
        "a", "a", "a", "←", "<<reset style>>", "<<reset style>>",
        "<<go to row: 2 col: 15>>", "2 ", "<<clear EOL>>", "bbb",
        "<<go to row: 3 col: 15>>", "3 ", "<<clear EOL>>", "ccc",
    ]);

    editor.go_to(1);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "1 ", "<<clear EOL>>", "aaa",
        "<<go to row: 2 col: 15>>", "2 ", "<<clear EOL>>", "<<reset style>>",
        "b", "b", "b", "⇆", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "3 ", "<<clear EOL>>", "ccc",
    ]);

    editor.go_to(2);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "2 ", "<<clear EOL>>", "bbb",
        "<<go to row: 2 col: 15>>", "3 ", "<<clear EOL>>", "<<reset style>>",
        "c", "c", "c", "⇄", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "4 ", "<<clear EOL>>", "ddd",
    ]);

    editor.go_to(3);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "3 ", "<<clear EOL>>", "ccc",
        "<<go to row: 2 col: 15>>", "4 ", "<<clear EOL>>", "<<reset style>>",
        "d", "d", "d", "®", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "5 ", "<<clear EOL>>", "ae"
    ]);

    editor.go_to(4);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "4 ", "<<clear EOL>>", "ddd",
        "<<go to row: 2 col: 15>>", "5 ", "<<clear EOL>>", "<<reset style>>",
        "a", "e", "◂", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "6 ", "<<clear EOL>>", "ddd"
    ]);

    editor.go_to(5);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "5 ", "<<clear EOL>>", "ae",
        "<<go to row: 2 col: 15>>", "6 ", "<<clear EOL>>", "<<reset style>>",
        "d", "d", "d", "▸", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "7 ", "<<clear EOL>>", "awes"
    ]);

    editor.go_to(6);
    editor.render(&mut gs);
    let drain = gs.backend().drain();

    #[rustfmt::skip]
    assert_eq!(drain.iter().map(|(_, txt)| txt.as_str()).collect::<Vec<_>>(), [
        "<<go to row: 1 col: 15>>", "6 ", "<<clear EOL>>", "ddd",
        "<<go to row: 2 col: 15>>", "7 ", "<<clear EOL>>", "<<reset style>>",
        "a", "w", "e", "s", "⭣", "<<reset style>>", "<<reset style>>",
        "<<go to row: 3 col: 15>>", "8 ", "<<clear EOL>>", "awdwe"
    ]);
}

#[test]
fn test_stripped_restricted_control_chars() {
    let cfg = IndentConfigs::default();
    let text = "aaa\r\tb\u{001B}\u{0008}bb\r\ncc\u{0008}c\n\rddd\u{001E}ae\u{001C}d\u{001B}dd\u{000C}awe\u{0008}s\u{000B}awd\u{001B}we";
    assert_eq!(
        LineParser::split_lines(text).into_editor_lines().into_iter().map(|eline| eline.unwrap()).collect::<Vec<_>>(),
        ["aaa", "\tbbb", "ccc", "ddd", "ae", "ddd", "awes", "awdwe"],
    );
    assert_eq!(
        LineParser::split_lines(text)
            .into_sanitzed_editor_lines(&cfg.indent)
            .into_iter()
            .map(|eline| eline.unwrap())
            .collect::<Vec<_>>(),
        ["aaa", "    bbb", "ccc", "ddd", "ae", "ddd", "awes", "awdwe"],
    )
}

#[test]
fn test_sanitize_text() {
    let indent_cfg = IndentConfigs::default();
    let result = LineParser::sanitize_text(
        "tesea\na\u{001B}\u{0008}dwa\r\nad\u{001B}wadw\u{000C}aw\u{0008}e\ts\u{000B}a\u{0008}wd\u{001B}we\r\nda\n\ra",
        indent_cfg.indent.as_str(),
    );
    assert_eq!(result, "tesea\nadwa\nadwadw\nawe    s\nawdwe\nda\na");
}

#[test]
fn test_parser_popup() {
    let indent_cfg = IndentConfigs::default();
    let parsed = LineParser::split_lines(
        "tesea\na\u{001B}\u{0008}dwa\r\nad\u{001B}wadw\u{000C}aw\u{0008}e\ts\u{000B}a\u{0008}wd\u{001B}we\r\nda\n\ra",
    );
    let mut popup = parsed.mock_popup(indent_cfg.indent.as_str());
    let mut gs = GlobalState::new(Rect::new(0, 0, 100, 20), CrossTerm::init());
    gs.force_area_calc();
    popup.render(&mut gs);
    let expect = [
        (ContentStyle::bold(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 4 col: 18>>".to_string()),
        (ContentStyle::bold(), "Found unexpected formatting:".to_string()),
        (ContentStyle::bold(), "<<padding: 32>>".to_string()),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::bold(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 5 col: 18>>".to_string()),
        (ContentStyle::bold(), "    Used tabs instead of space indent: present!".to_string()),
        (ContentStyle::bold(), "<<padding: 13>>".to_string()),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::bold(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 6 col: 18>>".to_string()),
        (ContentStyle::bold(), "    Used non posix line ends: present!".to_string()),
        (ContentStyle::bold(), "<<padding: 22>>".to_string()),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::bold(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 7 col: 18>>".to_string()),
        (ContentStyle::bold(), "Handle choices:".to_string()),
        (ContentStyle::bold(), "<<padding: 45>>".to_string()),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::reversed(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 8 col: 18>>".to_string()),
        (ContentStyle::reversed(), " >> sanitize".to_string()),
        (ContentStyle::reversed(), "<<padding: 48>>".to_string()),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 9 col: 18>>".to_string()),
        (ContentStyle::default(), "    do not sanitize - load as is".to_string()),
        (ContentStyle::default(), "<<padding: 28>>".to_string()),
        (ContentStyle::default(), "<<go to row: 10 col: 18>>".to_string()),
        (ContentStyle::default(), "    cancel".to_string()),
        (ContentStyle::default(), "<<padding: 50>>".to_string()),
        (ContentStyle::bold().with_fg(Color::Red), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 11 col: 18>>".to_string()),
        (
            ContentStyle::bold().with_fg(Color::Red),
            "Found U+001B ESC Control char -> will be stripped from text!".to_string(),
        ),
        (ContentStyle::default(), "<<set style>>".to_string()),
        (ContentStyle::bold().with_fg(Color::Red), "<<set style>>".to_string()),
        (ContentStyle::default(), "<<go to row: 12 col: 18>>".to_string()),
        (
            ContentStyle::bold().with_fg(Color::Red),
            "Found U+0008 BACKSPACE Control char -> will be stripped from".to_string(),
        ),
        (ContentStyle::default(), "<<set style>>".to_string()),
    ];
    assert_eq!(expect.as_slice(), gs.backend().drain());

    popup.next_option();
    popup.render(&mut gs);
    let writer_drain = gs.backend().drain();
    assert_eq!(writer_drain[21].1.as_str(), "    sanitize");
    assert_eq!(writer_drain[25].1.as_str(), " >> do not sanitize - load as is");
    assert_eq!(writer_drain[29].1.as_str(), "    cancel");

    popup.next_option();
    popup.render(&mut gs);
    let writer_drain = gs.backend().drain();
    assert_eq!(writer_drain[21].1.as_str(), "    sanitize");
    assert_eq!(writer_drain[24].1.as_str(), "    do not sanitize - load as is");
    assert_eq!(writer_drain[28].1.as_str(), " >> cancel");

    popup.prev_option();
    popup.render(&mut gs);
    let writer_drain = gs.backend().drain();
    assert_eq!(writer_drain[21].1.as_str(), "    sanitize");
    assert_eq!(writer_drain[25].1.as_str(), " >> do not sanitize - load as is");
    assert_eq!(writer_drain[29].1.as_str(), "    cancel");

    popup.clear_screen(&mut gs);
    let writer_drain = gs.backend().drain();
    let mut idx = 1;
    assert_eq!(36, writer_drain.len());
    loop {
        assert_eq!(writer_drain[idx].1, "<<padding: 84>>");
        idx += 2;
        if idx > writer_drain.len() {
            break;
        }
    }
}
