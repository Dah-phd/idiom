use crate::{
    components::{
        popups::editor_popups::go_to_line_popup,
        popups::{
            editor_popups::save_all_popup,
            tree_popups::{
                create_file_popup, find_paths_popup, rename_file_popup, select_file_popup, select_tree_file_popup,
            },
        },
        EditorState, EditorTerminal, Tree,
    },
    configs::{GeneralAction, KeyMap, Mode, PopupMessage},
};
use anyhow::Result;
use crossterm::event::Event;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{collections::HashMap, io::Stdout};
use tui::{backend::CrosstermBackend, Terminal};

const TICK: Duration = Duration::from_millis(100);

pub async fn app(terminal: &mut Terminal<CrosstermBackend<&Stdout>>, open_file: Option<PathBuf>) -> Result<()> {
    let configs = KeyMap::new();
    let mut mode = Mode::Select;
    let mut clock = Instant::now();
    let mut file_tree = Tree::new(open_file.is_none());
    let mut editor_state = EditorState::new(configs.editor_key_map());
    let mut general_key_map = configs.general_key_map();
    let mut tmux = EditorTerminal::new();
    let mut lsp_servers = HashMap::new();

    if let Some(path) = open_file {
        file_tree.select_by_path(&path);
        editor_state.new_from(path, &mut lsp_servers).await;
        mode = Mode::Insert;
    }

    drop(configs);

    loop {
        if matches!(mode, Mode::Select) {
            let _ = terminal.hide_cursor();
        }
        terminal.draw(|frame| {
            let mut screen = frame.size();
            screen = file_tree.render_with_remainder(frame, screen);
            screen = tmux.render_with_remainder(frame, screen);
            editor_state.render(frame, screen);
            mode.render_popup_if_exists(frame);
        })?;

        editor_state.lsp_updates().await;

        let timeout = TICK.checked_sub(clock.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if matches!(mode, Mode::Insert) && !tmux.active && editor_state.map(&key).await {
                    continue;
                }
                if let Some(msg) = mode.popup_map(&key) {
                    match msg {
                        PopupMessage::Exit => break,
                        PopupMessage::SaveAndExit => {
                            editor_state.save_all().await;
                            break;
                        }
                        PopupMessage::Open((path, line)) => {
                            file_tree.select_by_path(&path);
                            if !path.is_dir() {
                                editor_state.new_at_line(path, line, &mut lsp_servers).await;
                                mode = Mode::Insert;
                            } else {
                                mode = Mode::Select;
                            }
                            continue;
                        }
                        PopupMessage::SelectPath(pattern) => {
                            mode = Mode::Select.popup(Box::new(select_file_popup(file_tree.search_paths(pattern))));
                            continue;
                        }
                        PopupMessage::SelectFileLine(pattern) => {
                            mode =
                                Mode::Select.popup(Box::new(select_tree_file_popup(file_tree.search_files(pattern))));
                            continue;
                        }
                        PopupMessage::GoToLine(line_idx) => {
                            if let Some(editor) = editor_state.get_active() {
                                editor.go_to(line_idx)
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::CreateFileOrFolder(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder(name) {
                                if !new_path.is_dir() {
                                    editor_state.new_from(new_path, &mut lsp_servers).await;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::CreateFileOrFolderBase(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder_base(name) {
                                if !new_path.is_dir() {
                                    editor_state.new_from(new_path, &mut lsp_servers).await;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::RenameFile(name) => {
                            file_tree.rename_file(name);
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::None => continue,
                        PopupMessage::Done => {
                            mode = mode.clear_popup();
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
                    GeneralAction::FindInTree => {
                        mode = mode.popup(Box::new(find_paths_popup()));
                    }
                    GeneralAction::NewFile => {
                        mode = mode.popup(Box::new(create_file_popup(file_tree.get_first_selected_folder())));
                    }
                    GeneralAction::RenameFile => {
                        mode = mode.popup(Box::new(rename_file_popup(file_tree.get_first_selected_folder())));
                    }
                    GeneralAction::Expand => {
                        if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            editor_state.new_from(file_path, &mut lsp_servers).await;
                        }
                    }
                    GeneralAction::FinishOrSelect => {
                        if file_tree.on_open_tabs {
                            mode = Mode::Insert;
                        } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            if !file_path.is_dir() {
                                editor_state.new_from(file_path, &mut lsp_servers).await;
                                mode = Mode::Insert;
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
                    GeneralAction::Exit => {
                        if editor_state.are_updates_saved() && !matches!(mode, Mode::Popup(..)) {
                            break;
                        } else {
                            mode = mode.popup(Box::new(save_all_popup()))
                        }
                    }
                    GeneralAction::FileTreeModeOrCancelInput => mode = Mode::Select,
                    GeneralAction::SaveAll => editor_state.save().await,
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
                        mode = mode.popup(Box::new(go_to_line_popup()));
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
