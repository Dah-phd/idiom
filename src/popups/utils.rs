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

/// does not check for match between position and iter len
/// returned len is always 5
pub fn position_with_count_text<T>(index: usize, options: &[T]) -> String {
    let len = options.len();
    if len > 99 {
        return String::from("..99+");
    }

    if len == 0 {
        return String::from(" --- ");
    }

    let position = index + 1;
    if len < 10 {
        return format!("  {position}/{len}");
    }

    if position < 10 {
        return format!(" {position}/{len}");
    }

    format!("{position}/{len}")
}

/// returns the postion of current select
pub fn infer_word_search_positon(
    editor: &mut Editor,
    pattern: &mut TextField,
    buffer: &mut Vec<(CursorPosition, CursorPosition)>,
) -> Option<usize> {
    if editor.cursor.select_is_none() {
        editor.cursor.select_word(&editor.content);
    };
    let (from, to) = editor.cursor.select_get()?;
    if from.line != to.line {
        return None;
    }
    let word = editor.content.get(from.line).and_then(|line| line.get(from.char, to.char))?;
    pattern.text_set(word.to_owned());
    pattern.select_all();
    buffer.clear();
    editor.find(pattern.as_str(), buffer);
    buffer.iter().position(|(f, t)| f == &from && t == &to)
}

#[cfg(test)]
mod test {
    use super::{infer_word_search_positon, position_with_count_text};
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

    #[test]
    fn test_position_count_as_text() {
        assert_eq!(position_with_count_text(3, &[0; 100]), String::from("..99+"));
        assert_eq!(position_with_count_text(3, &[0; 20]), String::from(" 4/20"));
        assert_eq!(position_with_count_text(9, &[0; 99]), String::from("10/99"));
        assert_eq!(position_with_count_text(50, &[0; 99]), String::from("51/99"));
        assert_eq!(position_with_count_text(5, &[0; 9]), String::from("  6/9"));
    }
}
