use crate::{
    components::{
        popups::editor_popups::go_to_line_popup, popups::editor_popups::save_all_popup, EditorState, EditorTerminal,
        Tree,
    },
    configs::{GeneralAction, KeyMap, Mode, PopupMessage},
    lsp::LSP,
};
use anyhow::Result;
use crossterm::event::Event;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tui::{backend::Backend, Terminal};

const TICK: Duration = Duration::from_millis(100);

pub async fn app(terminal: &mut Terminal<impl Backend>, open_file: Option<PathBuf>) -> Result<()> {
    let configs = KeyMap::new();
    let mut mode = Mode::Select;
    let mut clock = Instant::now();
    let mut file_tree = Tree::new(open_file.is_none());
    let mut editor_state = EditorState::new(configs.editor_key_map());
    let mut lsp_servers: Vec<LSP> = vec![];
    let mut general_key_map = configs.general_key_map();
    let mut tmux = EditorTerminal::new().unwrap(); //TODO: Handle error variant
    if let Some(path) = open_file {
        editor_state.new_from(path).await;
        mode = Mode::Insert;
    }

    drop(configs);

    loop {
        if matches!(mode, Mode::Select) {
            let _ = terminal.hide_cursor();
        }
        terminal.draw(|frame| {
            let mut screen = file_tree.render(frame, frame.size());
            screen = tmux.render(frame, screen);
            editor_state.render(frame, screen);
            if let Mode::Popup((_, popup)) = &mut mode {
                popup.render(frame);
            }
        })?;

        let timeout = TICK
            .checked_sub(clock.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if matches!(mode, Mode::Insert) && !tmux.active && editor_state.map(&key) {
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
                        PopupMessage::GoToLine(line_idx) => {
                            if let Some(editor) = editor_state.get_active() {
                                editor.go_to(line_idx)
                            }
                            mode.clear_popup();
                            continue;
                        }
                    }
                }
                let action = if let Some(action) = general_key_map.map(&key) {
                    action
                } else {
                    continue;
                };
                if tmux.map(&action).await || file_tree.map(&action) {
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
                            editor_state.new_from(file_path).await;
                            if let Some(editor) = editor_state.get_active() {
                                if let Ok(lsp) = LSP::from(&editor.file_type).await {
                                    for req in lsp.responses.lock().unwrap().iter() {
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
                                editor_state.new_from(file_path).await;
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
                    GeneralAction::HideFileTree => file_tree.toggle(),
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
                    GeneralAction::GoToLinePopup if matches!(mode, Mode::Insert) => {
                        mode = mode.popup(go_to_line_popup());
                    }
                    GeneralAction::ToggleTerminal => {
                        tmux.toggle();
                    }
                    _ => (),
                }
            }
        }
        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    Ok(())
}
