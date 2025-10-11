use crate::workspace::{CursorPosition, Editor};
use idiom_tui::text_field::TextField;

pub fn next_option<T: Clone>(options: &[T], state: &mut usize) -> Option<T> {
    if options.is_empty() {
        *state = 0;
        return None;
    }
    if options.len() - 1 > *state {
        *state += 1;
    } else {
        *state = 0;
    }
    options.get(*state).cloned()
}

pub fn prev_option<T: Clone>(options: &[T], state: &mut usize) -> Option<T> {
    if options.is_empty() {
        *state = 0;
        return None;
    }
    if *state > 0 {
        *state -= 1;
    } else {
        *state = options.len() - 1;
    }
    options.get(*state).cloned()
}

pub fn count_as_string<T>(options: &[T]) -> String {
    let len = options.len();
    if len < 10 {
        format!("  {len}")
    } else if len < 100 {
        format!(" {len}")
    } else {
        String::from("99+")
    }
}

/// returns the postion of current select
pub fn infer_word_search_positon(
    editor: &mut Editor,
    pattern: &mut TextField,
    buffer: &mut Vec<(CursorPosition, CursorPosition)>,
) -> Option<usize> {
    if editor.cursor.select_is_none() {
        editor.select_token();
    };
    let (from, to) = editor.cursor.select_get()?;
    if from.line != to.line {
        return None;
    }
    let word = editor.content.get(from.line).and_then(|line| line.get(from.char, to.char))?;
    pattern.text_set(word.to_owned());
    buffer.clear();
    editor.find(pattern.as_str(), buffer);
    buffer.iter().position(|(f, t)| f == &from && t == &to)
}

#[cfg(test)]
mod test {
    use super::infer_word_search_positon;
    use crate::workspace::{editor::tests::mock_editor, CursorPosition};
    use idiom_tui::text_field::TextField;

    #[test]
    fn test_infer_word_search_position() {
        let mut editor = mock_editor(vec![
            "/// first line of data".into(),
            "let data = String::new();".into(),
            String::new(),
            "println!(\"{}\", data);".into(),
            "/// no ref to search".into(),
            String::new(),
            String::new(),
            "/// last ref to data".into(),
            String::new(),
        ]);
        editor.cursor.line = 3;
        editor.cursor.char = 18;
        let mut pattern = TextField::default();
        let mut buffer = vec![];
        let state = infer_word_search_positon(&mut editor, &mut pattern, &mut buffer);
        let expect_state = 2;
        assert_eq!(Some(expect_state), state);
        let expect = vec![
            (CursorPosition { line: 0, char: 18 }, CursorPosition { line: 0, char: 22 }),
            (CursorPosition { line: 1, char: 4 }, CursorPosition { line: 1, char: 8 }),
            (CursorPosition { line: 3, char: 15 }, CursorPosition { line: 3, char: 19 }),
            (CursorPosition { line: 7, char: 16 }, CursorPosition { line: 7, char: 20 }),
        ];
        assert_eq!(editor.cursor.select_get(), Some(expect[expect_state]));
        assert_eq!(buffer, expect);
    }
}
