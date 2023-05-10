mod components;
mod editor;
mod tree;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use tree::file_tree;
use tui::{backend::Backend, Terminal};

use crate::state::{State, Tree};

const TICK: Duration = Duration::from_millis(250);

pub fn app(terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
    let mut state = State::new();
    let mut clock = Instant::now();
    loop {
        terminal.draw(|frame| file_tree(frame, &mut state))?;

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
                        if matches!(key.code, KeyCode::Char('e')) || matches!(key.code, KeyCode::Char('E')) {
                            state.switch_tree()
                        }
                    }
                    KeyModifiers::NONE => match key.code {
                        KeyCode::Down | KeyCode::Char('d') | KeyCode::Char('D') => {
                            if let Some(tree) = &mut state.file_tree {
                                if let Some(numba) = tree.state.selected() {
                                    if numba < tree.tree.len() - 1 {
                                        tree.state.select(Some(numba + 1));
                                    } else {
                                        tree.state.select(Some(0))
                                    }
                                } else {
                                    tree.state.select(Some(0))
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                            if let Some(tree) = &mut state.file_tree {
                                if let Some(numba) = tree.state.selected() {
                                    if numba == 0 {
                                        tree.state.select(Some(tree.tree.len() - 1))
                                    } else {
                                        tree.state.select(Some(numba - 1))
                                    }
                                } else {
                                    tree.state.select(Some(tree.tree.len() - 1))
                                }
                            }
                        }
                        KeyCode::Left => {
                            if let Some(tree) = &mut state.file_tree {
                                if let Some(numba) = tree.state.selected() {
                                    if let Some(path) = tree.tree.get(numba) {
                                        tree.expanded.retain(|expanded_path| expanded_path != path)
                                    }
                                }
                            }
                        }
                        KeyCode::Right => {
                            if let Some(tree) = &mut state.file_tree {
                                expand_tree(tree)
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(tree) = &mut state.file_tree {
                                expand_tree(tree)
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

fn expand_tree(tree: &mut Tree) {
    if let Some(numba) = tree.state.selected() {
        if let Some(path) = tree.tree.get(numba) {
            tree.expanded.push(path.clone())
        }
    }
}
