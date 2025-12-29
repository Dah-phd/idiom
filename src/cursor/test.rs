use super::word::{PositionedWord, WordRange};
use super::{
    positions::{checked_select, checked_select_with_direction},
    CursorPosition,
};
use crate::{editor_line::EditorLine, utils::Direction};

fn match_char_range_to_word_range(range: WordRange, char_idx: usize, content: &[EditorLine]) {
    let char_range = WordRange::find_char_range(&content[range.line()], char_idx).unwrap();
    assert_eq!(range.from(), char_range.from);
    assert_eq!(range.to(), char_range.to);
}

#[test]
fn positioned_word_creation() {
    let content = vec![
        EditorLine::from("if word.starts_with(\"bird\") {"),
        EditorLine::from("    println!(\"🦀 end: {}\", &word);"),
        EditorLine::from("} // not a __word__"),
    ];
    let word = PositionedWord::find_at(&content, (0, 4).into()).unwrap();
    assert_eq!(word.as_str(), "word");
    assert_eq!(word.line(), 0);
    assert_eq!(word.from(), 3);
    assert_eq!(word.to(), 7);

    let word = PositionedWord::find_at(&content, (1, 28).into()).unwrap();
    assert_eq!(word.as_str(), "word");
    assert_eq!(word.line(), 1);
    assert_eq!(word.from(), 27);
    assert_eq!(word.to(), 31);
}

#[test]
fn test_word_range_at() {
    let content = vec![EditorLine::from("let word = \"word\";")];
    let char_idx = 4;
    let line_idx = 0;
    let wr = WordRange::find_at(&content, (line_idx, char_idx).into()).unwrap();
    assert_eq!(wr, WordRange::new_checked(line_idx, 4, 8).unwrap());
    assert_eq!(&content[line_idx][wr.char_range()], "word");
    match_char_range_to_word_range(wr, char_idx, &content);
    let content = vec![EditorLine::from("let __word__ = \"word\";")];
    let wr = WordRange::find_at(&content, (line_idx, char_idx).into()).unwrap();
    assert_eq!(wr, WordRange::new_checked(line_idx, 4, 12).unwrap());
    assert_eq!(&content[line_idx][wr.char_range()], "__word__");
    match_char_range_to_word_range(wr, char_idx, &content);
    let content = vec![EditorLine::from("let (__word__,) = \"word\";")];
    let wr = WordRange::find_at(&content, (line_idx, char_idx).into());
    assert!(wr.is_none());
    assert_eq!(None, WordRange::find_char_range(&content[0], char_idx));
}

#[test]
fn test_iter_word_selects() {
    let content = vec![
        EditorLine::from("let word = \"bird\";"),
        EditorLine::from("println!(\"{:?}\", &word);"),
        EditorLine::from("let is_there = word.contins(\"word\");"),
        EditorLine::from("if word.starts_with(\"bird\") {"),
        EditorLine::from("    println!(\"🦀 end: {}\", &word);"),
        EditorLine::from("} // not a __word__"),
    ];

    let word = PositionedWord::find_at(&content, (0, 4).into()).unwrap();

    let content_iter = content.iter().enumerate().skip(3).chain(content.iter().enumerate().take(3));
    let selects = word.iter_word_ranges(content_iter).collect::<Vec<_>>();
    let line_order = [3, 4, 0, 1, 2, 2];
    assert_eq!(selects.len(), line_order.len());
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range.line(), line_order[idx]);
        assert_eq!(range.line(), line_order[idx]);
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }
}

#[test]
fn test_find_word_inline_from() {
    let content = vec![
        EditorLine::from("let word = String::from(\"word\"); // word and __word__"),
        EditorLine::from("let word = String::from(\"word\"); // 🦀 word and __word__"),
        EditorLine::from("println!(\"end 🦀 {}\", word) // word"),
        EditorLine::from("println!(\"end 🦀 {}\", word) // __word__"),
    ];

    let word = PositionedWord::find_at(&content, (0, 26).into()).unwrap();
    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        assert_eq!(range, WordRange::new_checked(0, 36, 40).unwrap());
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }

    let word = PositionedWord::find_at(&content, (1, 4).into()).unwrap();

    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange::new_checked(1, 25, 29).unwrap(),
        WordRange::new_checked(1, 38, 42).unwrap(),
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        // only 2 offset for the emoji + /s
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }

    let word = PositionedWord::find_at(&content, (2, 23).into()).unwrap();

    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        // only 2 offset for the emoji + /s
        assert_eq!(range, WordRange::new_checked(2, 30, 34).unwrap());
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }

    let word = PositionedWord::find_at(&content, (3, 23).into()).unwrap();

    let selects = word.find_word_inline_after(&content);
    assert!(selects.is_none() || selects.unwrap().collect::<Vec<_>>().is_empty());
}

#[test]
fn test_find_word_inline_to() {
    let content = vec![
        EditorLine::from("let word = String::from(\"word\"); // word and __word__"),
        EditorLine::from("let word = String::from(\"word\"); // 🦀 word and __word__"),
        EditorLine::from("println!(\"word 🦀 {}\", word) // word"),
    ];

    let word = PositionedWord::find_at(&content, (0, 26).into()).unwrap();

    let selects = word.find_word_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        assert_eq!(range, WordRange::new_checked(0, 4, 8).unwrap());
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
    }

    let word = PositionedWord::find_at(&content, (1, 40).into()).unwrap();

    let selects = word.find_word_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange::new_checked(1, 4, 8).unwrap(),
        WordRange::new_checked(1, 25, 29).unwrap(),
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }

    let word = PositionedWord::find_at(&content, (2, 33).into()).unwrap();

    let selects = word.find_word_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange::new_checked(2, 10, 14).unwrap(),
        WordRange::new_checked(2, 22, 26).unwrap(),
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line()].get(range.from(), range.to()), Some(word.as_str()));
        match_char_range_to_word_range(range, range.from(), &content);
    }
}

#[test]
fn test_checked_select() {
    let pos = CursorPosition { line: 1, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 0 };
    assert!(pos < pos2);
    let select = checked_select(pos, pos2);
    assert_eq!(Some((pos, pos2)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 1, char: 0 };
    assert!(pos > pos2);
    let select = checked_select(pos, pos2);
    assert_eq!(Some((pos2, pos)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 5 };
    assert!(pos < pos2);
    let select = checked_select(pos, pos2);
    assert_eq!(Some((pos, pos2)), select);

    let pos = CursorPosition { line: 2, char: 10 };
    let pos2 = CursorPosition { line: 2, char: 5 };
    assert!(pos > pos2);
    let select = checked_select(pos, pos2);
    assert_eq!(Some((pos2, pos)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 0 };
    assert!(pos == pos2);
    let select = checked_select(pos, pos2);
    assert_eq!(None, select);
}

#[test]
fn test_checked_select_direction() {
    let pos = CursorPosition { line: 1, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 0 };
    assert!(pos < pos2);
    let select = checked_select_with_direction(pos, pos2);
    assert_eq!(Some(((pos, pos2), Direction::Normal)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 1, char: 0 };
    assert!(pos > pos2);
    let select = checked_select_with_direction(pos, pos2);
    assert_eq!(Some(((pos2, pos), Direction::Reversed)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 5 };
    assert!(pos < pos2);
    let select = checked_select_with_direction(pos, pos2);
    assert_eq!(Some(((pos, pos2), Direction::Normal)), select);

    let pos = CursorPosition { line: 2, char: 5 };
    let pos2 = CursorPosition { line: 2, char: 1 };
    assert!(pos > pos2);
    let select = checked_select_with_direction(pos, pos2);
    assert_eq!(Some(((pos2, pos), Direction::Reversed)), select);

    let pos = CursorPosition { line: 2, char: 0 };
    let pos2 = CursorPosition { line: 2, char: 0 };
    assert!(pos == pos2);
    let select = checked_select_with_direction(pos, pos2);
    assert_eq!(None, select);
}

#[test]
fn test_word_range() {
    let content = vec![
        EditorLine::from("let word = String::from(\"word\"); // word and __word__"),
        EditorLine::from("let word = String::from(\"word\"); // 🦀 word and __word__"),
        EditorLine::from("println!(\"word 🦀 {}\", word) // word"),
    ];

    let selects = [
        WordRange::find_at(&content, (0, 26).into()).unwrap(),
        WordRange::find_at(&content, (1, 40).into()).unwrap(),
        WordRange::find_at(&content, (2, 33).into()).unwrap(),
    ];

    for range in selects {
        let word = range.get_text(&content).unwrap();
        let unchecked = range.get_text_uncheded(&content);
        assert_eq!(word, unchecked);
        let (from, to) = range.as_select();
        assert_eq!(from.line, to.line);
        assert_eq!(range.line(), from.line);
        assert_eq!(range.from(), from.char);
        assert_eq!(range.to(), to.char);
        assert_eq!(word, content[from.line].get(from.char, to.char).unwrap());
        match_char_range_to_word_range(range, range.from(), &content);
    }
}

#[test]
fn test_cursor_pos_next() {
    let content = vec![
        EditorLine::from(""),
        EditorLine::from(""),
        EditorLine::from(""),
        EditorLine::from("123"),
        EditorLine::from(""),
    ];
    let cur = CursorPosition { line: 0, char: 0 };
    let next = cur.next(&content).unwrap();
    assert_eq!(next, CursorPosition { line: 3, char: 1 });
    let next = next.next(&content).unwrap();
    assert_eq!(next, CursorPosition { line: 3, char: 2 });
    let next = next.next(&content).unwrap();
    assert_eq!(next, CursorPosition { line: 3, char: 3 });
    assert_eq!(next.next(&content), None);
}

#[test]
fn test_cursor_pos_prev() {
    let content = vec![
        EditorLine::from(""),
        EditorLine::from("123"),
        EditorLine::from(""),
        EditorLine::from(""),
        EditorLine::from(""),
    ];
    let cur = CursorPosition { line: 4, char: 0 };
    let prev = cur.prev(&content).unwrap();
    assert_eq!(prev, CursorPosition { line: 1, char: 2 });
    let prev = prev.prev(&content).unwrap();
    assert_eq!(prev, CursorPosition { line: 1, char: 1 });
    let prev = prev.prev(&content).unwrap();
    assert_eq!(prev, CursorPosition { line: 1, char: 0 });
    assert_eq!(prev.prev(&content), None);
}
