use std::time::{Duration, Instant};

use crate::{
    components::{popups::editor_popups::save_all_popup, EditorState, Tree},
    lsp::LSP,
    messages::{Mode, PopupMessage},
};
use crossterm::event::{Event, KeyCode, KeyModifiers};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

#[cfg(target_os = "windows")]
use crossterm::event::KeyEventKind;

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
            if let Mode::Popup((_, popup)) = &mut mode {
                popup.render(frame);
            }
        })?;

        if matches!(mode, Mode::Select) {
            let _ = terminal.hide_cursor();
        }

        let timeout = TICK
            .checked_sub(clock.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                #[cfg(target_os = "windows")]
                if matches!(key.kind, KeyEventKind::Release) {
                    continue;
                }
                if matches!(
                    key.code,
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('q') | KeyCode::Char('Q')
                ) && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    if editor_state.are_updates_saved() && !matches!(mode, Mode::Popup(..)) {
                        break;
                    } else {
                        mode = mode.popup(save_all_popup())
                    }
                };
                if match &mut mode {
                    Mode::Insert => editor_state.map(&key),
                    Mode::Select => file_tree.map(&key),
                    Mode::Popup((_, popup)) => match popup.map(&key) {
                        PopupMessage::None => continue,
                        PopupMessage::Exit => break,
                        PopupMessage::Done => {
                            mode.clear_popup();
                            true
                        }
                        PopupMessage::SaveAndExit => {
                            editor_state.save_all();
                            break;
                        }
                    },
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
