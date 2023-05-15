use std::time::{Duration, Instant};

use crate::{
    components::{EditorState, Tree},
    messages::Mode,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

const TICK: Duration = Duration::from_millis(250);

pub fn app(terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
    let mut mode = Mode::Select;
    let mut clock = Instant::now();
    let mut file_tree = Tree::default();
    let mut editor_state = EditorState::default();
    loop {
        terminal.draw(|frame| {
            let screen_areas = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(15), Constraint::Min(2)].as_ref())
                .split(frame.size());
            file_tree.render(frame, screen_areas[0]);
            editor_state.render(frame, screen_areas[1]);
        })?;
        match mode {
            Mode::Insert => {}
            Mode::Select => {
                let _ = terminal.hide_cursor();
            }
            Mode::Popup => {
                let _ = terminal.hide_cursor();
            }
        }

        let timeout = TICK
            .checked_sub(clock.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                match mode {
                    Mode::Insert => insert_mode(&key, &mut file_tree, &mut editor_state),
                    Mode::Select => {
                        if select_mode(&key, &mut file_tree, &mut editor_state) {
                            break;
                        }
                    }
                    Mode::Popup => {}
                }
                if matches!(key.code, KeyCode::Enter) {
                    mode = Mode::Insert
                }
                if matches!(key.code, KeyCode::Esc) {
                    mode = Mode::Select
                }
            }
        }
        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    Ok(())
}

fn insert_mode(key: &KeyEvent, file_tree: &mut Tree, editor_state: &mut EditorState) {
    if let Some(editor) = editor_state.get_active() {
        if matches!(key.modifiers, KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s') | KeyCode::Char('S'))
        {
            editor.save();
            return;
        }
        match key.code {
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    editor.scroll_up()
                } else {
                    editor.navigate_up()
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    editor.scroll_down()
                } else {
                    editor.navigate_down()
                }
            }
            KeyCode::Left => editor.navigate_left(),
            KeyCode::Right => editor.navigate_right(),
            KeyCode::Char(c) => editor.push_str(c.to_string().as_str()),
            KeyCode::Backspace => editor.backspace(),
            KeyCode::Enter => editor.new_line(),
            KeyCode::Tab => editor.indent(),
            KeyCode::Delete => editor.del(),
            _ => {}
        }
    }
}

fn select_mode(key: &KeyEvent, file_tree: &mut Tree, editor_state: &mut EditorState) -> bool {
    match key.modifiers {
        KeyModifiers::CONTROL => {
            if matches!(
                key.code,
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('q') | KeyCode::Char('Q')
            ) {
                return true;
            }
        }

        KeyModifiers::SHIFT => if matches!(key.code, KeyCode::Char('e')) || matches!(key.code, KeyCode::Char('E')) {},

        KeyModifiers::NONE => match key.code {
            KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(numba) = file_tree.state.selected() {
                    if numba < file_tree.tree.len() - 1 {
                        file_tree.state.select(Some(numba + 1));
                    } else {
                        file_tree.state.select(Some(0))
                    }
                } else {
                    file_tree.state.select(Some(0))
                }
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                if let Some(numba) = file_tree.state.selected() {
                    if numba == 0 {
                        file_tree.state.select(Some(file_tree.tree.len() - 1))
                    } else {
                        file_tree.state.select(Some(numba - 1))
                    }
                } else {
                    file_tree.state.select(Some(file_tree.tree.len() - 1))
                }
            }
            KeyCode::Left => {
                if let Some(numba) = file_tree.state.selected() {
                    if let Some(path) = file_tree.tree.get(numba) {
                        file_tree.expanded.retain(|expanded_path| expanded_path != path)
                    }
                }
            }
            KeyCode::Right => {
                if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                    editor_state.new_from(file_path);
                }
            }
            KeyCode::Enter => {
                if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                    editor_state.new_from(file_path);
                }
            }
            KeyCode::Tab => {
                if let Some(editor_id) = editor_state.state.selected() {
                    if editor_id >= editor_state.editors.len() - 1 {
                        editor_state.state.select(Some(0))
                    } else {
                        editor_state.state.select(Some(editor_id + 1))
                    }
                }
            }
            _ => {}
        },

        _ => {}
    }
    false
}
