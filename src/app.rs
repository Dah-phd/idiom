use crate::{
    components::{
        popups::editor_popups::{go_to_line_popup, select_editor_popup},
        popups::{editor_popups::rename_var_popup, message},
        popups::{
            editor_popups::{find_in_editor_popup, save_all_popup, select_line_popup},
            tree_popups::{
                create_file_popup, find_in_tree_popup, rename_file_popup, select_file_popup, select_tree_file_popup,
            },
        },
        EditorState, EditorTerminal, Footer, Tree,
    },
    configs::{GeneralAction, KeyMap, Mode, PopupMessage},
};
use anyhow::Result;
use crossterm::event::Event;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(100);

pub async fn app(terminal: &mut Terminal<CrosstermBackend<&Stdout>>, open_file: Option<PathBuf>) -> Result<()> {
    let configs = KeyMap::new();
    let mut clock = Instant::now();
    let mut mode = Mode::Select;
    let mut general_key_map = configs.general_key_map();

    // COMPONENTS
    let mut file_tree = Tree::new(open_file.is_none());
    let mut editor_state = EditorState::from(configs.editor_key_map());
    let mut footer = Footer::default();
    let mut tmux = EditorTerminal::new();

    // CLI SETUP
    if let Some(path) = open_file {
        file_tree.select_by_path(&path);
        editor_state.new_from(path).await;
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
            screen = footer.render_with_remainder(frame, screen, &mode, editor_state.get_stats());
            screen = tmux.render_with_remainder(frame, screen);
            editor_state.render(frame, screen);
            mode.render_popup_if_exists(frame);
        })?;

        editor_state.lexer_updates().await;

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
                                editor_state.new_at_line(path, line).await;
                                mode = Mode::Insert;
                            } else {
                                mode = Mode::Select;
                            }
                            continue;
                        }
                        PopupMessage::ActivateEditor(idx) => {
                            editor_state.state.select(Some(idx));
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::SelectPath(pattern) => {
                            mode = Mode::Select.popup(select_file_popup(file_tree.search_select_paths(pattern)));
                            continue;
                        }
                        PopupMessage::SelectPathFull(pattern) => {
                            mode = Mode::Select.popup(select_file_popup(file_tree.search_paths(pattern)));
                            continue;
                        }
                        PopupMessage::SelectTreeFiles(pattern) => {
                            mode = Mode::Select
                                .popup(select_tree_file_popup(file_tree.search_select_files(pattern).await));
                            continue;
                        }
                        PopupMessage::SelectTreeFilesFull(pattern) => {
                            mode = Mode::Select.popup(select_tree_file_popup(file_tree.search_files(pattern).await));
                            continue;
                        }
                        PopupMessage::SelectOpenedFile(pattern) => {
                            if let Some(editor) = editor_state.get_active() {
                                mode = Mode::Insert.popup(select_line_popup(editor.search_file(&pattern)));
                            } else {
                                mode = mode.clear_popup();
                            }
                            continue;
                        }
                        PopupMessage::GoToSelect(select) => {
                            if let Some(editor) = editor_state.get_active() {
                                editor.go_to_select(select);
                            } else {
                                mode = mode.clear_popup();
                            }
                            continue;
                        }
                        PopupMessage::GoToLine(line_idx) => {
                            if let Some(editor) = editor_state.get_active() {
                                editor.go_to(line_idx)
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::UpdateEditor => {
                            mode.update_editor(&mut editor_state);
                            continue;
                        }
                        PopupMessage::UpdateFooter => {
                            mode.update_footer(&mut footer);
                            continue;
                        }
                        PopupMessage::UpdateTree => {
                            mode.update_tree(&mut file_tree);
                            continue;
                        }
                        PopupMessage::CreateFileOrFolder(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder(name) {
                                if !new_path.is_dir() {
                                    editor_state.new_from(new_path).await;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::CreateFileOrFolderBase(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder_base(name) {
                                if !new_path.is_dir() {
                                    editor_state.new_from(new_path).await;
                                    mode = Mode::Insert;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::Rename(new_name) => {
                            mode = mode.clear_popup();
                            editor_state.renames(new_name).await;
                            continue;
                        }
                        PopupMessage::RenameFile(name) => {
                            mode = mode.clear_popup();
                            if let Err(error) = file_tree.rename_file(name) {
                                mode = mode.popup(Box::new(message(error.to_string())))
                            }
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
                    GeneralAction::Find => {
                        if matches!(mode, Mode::Insert) {
                            mode = mode.popup(find_in_editor_popup());
                        } else {
                            mode = mode.popup(find_in_tree_popup());
                        }
                    }
                    GeneralAction::SelectOpenEditor => {
                        mode = mode.popup(select_editor_popup(editor_state.tabs()));
                    }
                    GeneralAction::NewFile => {
                        mode = mode.popup(create_file_popup(file_tree.get_first_selected_folder_display()));
                    }
                    GeneralAction::Rename => match mode {
                        Mode::Insert => {
                            mode = mode.popup(rename_var_popup());
                        }
                        Mode::Select => {
                            if let Some(tree_path) = file_tree.get_selected() {
                                mode = mode.popup(rename_file_popup(tree_path.path().display().to_string()));
                            }
                        }
                        _ => (),
                    },
                    GeneralAction::Expand => {
                        if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            editor_state.new_from(file_path).await;
                        }
                    }
                    GeneralAction::FinishOrSelect => {
                        if file_tree.on_open_tabs {
                            mode = Mode::Insert;
                        } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            if !file_path.is_dir() {
                                editor_state.new_from(file_path).await;
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
                            mode = mode.popup(save_all_popup());
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
