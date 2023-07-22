use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    components::{popups::editor_popups::save_all_popup, EditorState, Tree},
    lsp::LSP,
    messages::{GeneralAction, KeyMap, Mode, PopupMessage},
};
use crossterm::event::Event;

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};

const TICK: Duration = Duration::from_millis(250);

pub async fn app(terminal: &mut Terminal<impl Backend>, open_file: Option<PathBuf>) -> std::io::Result<()> {
    let configs = KeyMap::new();
    let mut mode = Mode::Select;
    let mut clock = Instant::now();
    let mut file_tree = Tree::default();
    let mut hide_file_tree = false;
    let mut editor_state = EditorState::new(configs.editor_key_map());
    let mut lsp_servers: Vec<LSP> = vec![];
    let mut general_key_map = configs.general_key_map();
    if let Some(path) = open_file {
        editor_state.new_from(path);
        mode = Mode::Insert;
        hide_file_tree = true;
    }

    drop(configs);

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
                if matches!(mode, Mode::Insert) && editor_state.map(&key) {
                    continue;
                }
                if let Mode::Popup((_, popup)) = &mut mode {
                    match popup.map(&key) {
                        PopupMessage::None => continue,
                        PopupMessage::Exit => break,
                        PopupMessage::Done => {
                            mode.clear_popup();
                            continue;
                        }
                        PopupMessage::SaveAndExit => {
                            editor_state.save_all();
                            break;
                        }
                    }
                }
                let action = if let Some(action) = general_key_map.map(&key) {
                    action
                } else {
                    continue;
                };
                if file_tree.map(&action) {
                    continue;
                }
                match action {
                    GeneralAction::Exit => {
                        if editor_state.are_updates_saved() && !matches!(mode, Mode::Popup(..)) {
                            break;
                        } else {
                            mode = mode.popup(save_all_popup())
                        }
                    }
                    GeneralAction::Expand => {
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
                    GeneralAction::FinishOrSelect => {
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
                    GeneralAction::NextTab => {
                        if let Some(editor_id) = editor_state.state.selected() {
                            file_tree.on_open_tabs = true;
                            if editor_id >= editor_state.editors.len() - 1 {
                                editor_state.state.select(Some(0))
                            } else {
                                editor_state.state.select(Some(editor_id + 1))
                            }
                        }
                    }
                    GeneralAction::FileTreeModeOrCancelInput => mode = Mode::Select,
                    GeneralAction::SaveAll => editor_state.save(),
                    GeneralAction::HideFileTree => hide_file_tree = !hide_file_tree,
                    GeneralAction::PreviousTab => {
                        if let Some(editor_id) = editor_state.state.selected() {
                            file_tree.on_open_tabs = true;
                            if editor_id == 0 {
                                editor_state.state.select(Some(editor_state.editors.len() - 1))
                            } else {
                                editor_state.state.select(Some(editor_id - 1))
                            }
                        }
                    }
                    GeneralAction::RefreshSettings => {
                        let new_key_map = KeyMap::new();
                        general_key_map = new_key_map.general_key_map();
                        editor_state.refresh_cfg(new_key_map.editor_key_map());
                    }
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
