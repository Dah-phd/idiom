use std::time::{Duration, Instant};

use crate::components::{EditorState, Tree};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

const TICK: Duration = Duration::from_millis(250);

pub fn app(terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
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

        let timeout = TICK
            .checked_sub(clock.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key.modifiers {
                    KeyModifiers::CONTROL => {
                        if matches!(key.code, KeyCode::Char('d')) {
                            break;
                        }
                    }
                    KeyModifiers::SHIFT => {
                        if matches!(key.code, KeyCode::Char('e')) || matches!(key.code, KeyCode::Char('E')) {}
                    }
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
            }
        }

        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    Ok(())
}
