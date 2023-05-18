use std::time::{Duration, Instant};

use crate::{
    components::{EditorState, Tree},
    messages::Mode,
};
use crossterm::event::{Event, KeyCode, KeyModifiers};
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
                if matches!(
                    key.code,
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('q') | KeyCode::Char('Q')
                ) && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                };
                if match mode {
                    Mode::Insert => editor_state.map(&key),
                    Mode::Select => file_tree.map(&key),
                    Mode::Popup => true,
                } {
                    continue;
                }
                match key.modifiers {
                    KeyModifiers::NONE => match key.code {
                        KeyCode::Right => {
                            if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                                editor_state.new_from(file_path);
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                                if !file_path.is_dir() {
                                    editor_state.new_from(file_path);
                                    mode = Mode::Insert
                                }
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
                    KeyModifiers::CONTROL => match key.code {
                        KeyCode::Char('s') | KeyCode::Char('S') => editor_state.save(),
                        _ => {}
                    },
                    _ => {}
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
