use super::word::WordRange;
use crate::workspace::EditorLine;

#[test]
fn test_word_range_at() {
    let content = vec![EditorLine::from("let word = \"word\";")];
    let wr = WordRange::find_at(&content, (0, 4).into()).unwrap();
    assert_eq!(wr, WordRange { line: 0, from: 4, to: 8 });
    assert_eq!(wr.get_text(&content), Some("word"));
    let content = vec![EditorLine::from("let __word__ = \"word\";")];
    let wr = WordRange::find_at(&content, (0, 4).into()).unwrap();
    assert_eq!(wr, WordRange { line: 0, from: 4, to: 12 });
    assert_eq!(wr.get_text(&content), Some("__word__"));
    let content = vec![EditorLine::from("let (__word__,) = \"word\";")];
    let wr = WordRange::find_at(&content, (0, 4).into());
    assert!(wr.is_none());
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

    let word_range = WordRange::find_at(&content, (0, 4).into()).unwrap();
    let word = word_range.into_word(&content).unwrap();

    let content_iter = content.iter().enumerate().skip(3).chain(content.iter().enumerate().take(3));
    let selects = word.iter_word_selects(content_iter).collect::<Vec<_>>();
    let line_order = [3, 4, 0, 1, 2, 2];
    assert_eq!(selects.len(), line_order.len());
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range.line, line_order[idx]);
        assert_eq!(range.line, line_order[idx]);
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
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

    let range = WordRange::find_at(&content, (0, 26).into()).unwrap();
    let word = range.into_word(&content).unwrap();
    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        assert_eq!(range, WordRange { line: 0, from: 36, to: 40 });
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }

    let range = WordRange::find_at(&content, (1, 4).into()).unwrap();
    let word = range.into_word(&content).unwrap();

    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange { line: 1, from: 25, to: 29 },
        WordRange { line: 1, from: 38, to: 42 },
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        // only 2 offset for the emoji + /s
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }

    let range = WordRange::find_at(&content, (2, 23).into()).unwrap();
    let word = range.into_word(&content).unwrap();

    let selects = word.find_word_inline_after(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        // only 2 offset for the emoji + /s
        assert_eq!(range, WordRange { line: 2, from: 30, to: 34 });
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }

    let range = WordRange::find_at(&content, (3, 23).into()).unwrap();
    let word = range.into_word(&content).unwrap();

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

    let word = WordRange::find_at(&content, (0, 26).into()).and_then(|r| r.into_word(&content)).unwrap();

    let selects = word.find_words_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 1);
    for range in selects {
        assert_eq!(range, WordRange { line: 0, from: 4, to: 8 });
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }

    let word = WordRange::find_at(&content, (1, 40).into()).and_then(|r| r.into_word(&content)).unwrap();

    let selects = word.find_words_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange { line: 1, from: 4, to: 8 },
        WordRange { line: 1, from: 25, to: 29 },
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }

    let word = WordRange::find_at(&content, (2, 33).into()).and_then(|r| r.into_word(&content)).unwrap();

    let selects = word.find_words_inline_before(&content).unwrap().collect::<Vec<_>>();
    assert_eq!(selects.len(), 2);
    let expected = [
        WordRange { line: 2, from: 10, to: 14 },
        WordRange { line: 2, from: 22, to: 26 },
    ];
    for (idx, range) in selects.into_iter().enumerate() {
        assert_eq!(range, expected[idx]);
        assert_eq!(content[range.line].get(range.from, range.to), Some(word.as_str()));
    }
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
        let (from, to) = range.as_select();
        assert_eq!(from.line, to.line);
        assert_eq!(range.line, from.line);
        assert_eq!(range.from, from.char);
        assert_eq!(range.to, to.char);
        assert_eq!(word, content[from.line].get(from.char, to.char).unwrap());
    }
}
