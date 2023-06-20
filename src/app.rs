use std::time::{Duration, Instant};

use crate::{
    components::{EditorState, Tree},
    lsp::LSP,
    messages::Mode,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

const TICK: Duration = Duration::from_millis(250);

pub async fn app(terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
    let mut mode = Mode::Select;
    let mut clock = Instant::now();
    let mut file_tree = Tree::default();
    let mut hide_file_tree = false;
    let mut editor_state = EditorState::default();
    let mut lsp_servers: Vec<LSP> = vec![];

    loop {
        terminal.draw(|frame| {
            let screen_areas = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(if matches!(mode, Mode::Select) || !hide_file_tree {
                        15
                    } else {
                        0
                    }),
                    Constraint::Min(2),
                ])
                .split(frame.size());
            if matches!(mode, Mode::Select) || !hide_file_tree {
                file_tree.render(frame, screen_areas[0]);
            }
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
            // TODO combine all controls in here!, should be easier to handle if something needs changin - this iwill be main
            if let Event::Key(key) = crossterm::event::read()? {
                if matches!(key.kind, KeyEventKind::Release) {
                    continue;
                }
                if matches!(
                    key.code,
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('q') | KeyCode::Char('Q')
                ) && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    if editor_state.are_updates_saved() {
                        break;
                    } else {
                        panic!("WORKS!")
                    }
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
                                if let Some(editor) = editor_state.get_active() {
                                    if let Ok(lsp) = LSP::from(&editor.file_type).await {
                                        for req in lsp.que.lock().unwrap().iter() {
                                            let _ = req;
                                        }
                                        lsp_servers.push(lsp);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if file_tree.on_open_tabs {
                                mode = Mode::Insert;
                            } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                                if !file_path.is_dir() {
                                    editor_state.new_from(file_path);
                                    mode = Mode::Insert;
                                    if let Some(editor) = editor_state.get_active() {
                                        if let Ok(lsp) = LSP::from(&editor.file_type).await {
                                            lsp_servers.push(lsp)
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Tab => {
                            if let Some(editor_id) = editor_state.state.selected() {
                                file_tree.on_open_tabs = true;
                                if editor_id >= editor_state.len() - 1 {
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
                        KeyCode::Char('e') | KeyCode::Char('E') => hide_file_tree = !hide_file_tree,
                        KeyCode::Tab => {
                            if let Some(editor_id) = editor_state.state.selected() {
                                file_tree.on_open_tabs = true;
                                if editor_id == 0 {
                                    editor_state.state.select(Some(editor_state.len() - 1))
                                } else {
                                    editor_state.state.select(Some(editor_id - 1))
                                }
                            }
                        }
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
