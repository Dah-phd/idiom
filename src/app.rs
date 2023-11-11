use crate::{
    components::{
        popups::editor_popups::{editor_selector, go_to_line_popup},
        popups::{
            editor_popups::{find_in_editor_popup, save_all_popup, select_selector},
            tree_popups::{
                create_file_popup, file_selector, find_in_tree_popup, rename_file_popup, tree_file_selector,
            },
        },
        popups::{
            editor_popups::{rename_var_popup, replace_in_editor_popup},
            message,
        },
        EditorTerminal, Footer, Tree, Workspace,
    },
    configs::{GeneralAction, KeyMap, Mode},
    events::messages::PopupMessage,
    events::Events,
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
    let mut events = Events::default();

    // COMPONENTS
    let mut file_tree = Tree::new(open_file.is_none());
    let mut workspace = Workspace::from(configs.editor_key_map());
    let mut footer = Footer::default();
    let mut tmux = EditorTerminal::new();

    // CLI SETUP
    if let Some(path) = open_file {
        file_tree.select_by_path(&path);
        workspace.new_from(path).await;
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
            screen = footer.render_with_remainder(frame, screen, &mode, workspace.get_stats());
            screen = tmux.render_with_remainder(frame, screen);
            workspace.render(frame, screen);
            mode.render_popup_if_exists(frame);
        })?;

        workspace.lexer_updates().await;

        let timeout = TICK.checked_sub(clock.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if !tmux.active && workspace.map(&key, &mut mode).await {
                    continue;
                }
                if let Some(msg) = mode.popup_map(&key) {
                    match msg {
                        PopupMessage::Exit => break,
                        PopupMessage::SaveAndExit => {
                            workspace.save_all().await;
                            break;
                        }
                        PopupMessage::Open(path, line) => {
                            file_tree.select_by_path(&path);
                            if !path.is_dir() {
                                workspace.new_at_line(path, line).await;
                                mode = Mode::Insert;
                            } else {
                                mode = Mode::Select;
                            }
                            continue;
                        }
                        PopupMessage::ActivateEditor(idx) => {
                            workspace.state.select(Some(idx));
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::SelectPath(pattern) => {
                            mode = Mode::Select.popup(file_selector(file_tree.search_select_paths(pattern)));
                            continue;
                        }
                        PopupMessage::SelectPathFull(pattern) => {
                            mode = Mode::Select.popup(file_selector(file_tree.search_paths(pattern)));
                            continue;
                        }
                        PopupMessage::SelectTreeFiles(pattern) => {
                            mode = Mode::Select.popup(tree_file_selector(file_tree.search_select_files(pattern).await));
                            continue;
                        }
                        PopupMessage::SelectTreeFilesFull(pattern) => {
                            mode = Mode::Select.popup(tree_file_selector(file_tree.search_files(pattern).await));
                            continue;
                        }
                        PopupMessage::SelectOpenedFile(pattern) => {
                            if let Some(editor) = workspace.get_active() {
                                mode = Mode::Insert.popup(select_selector(editor.find_with_line(&pattern)));
                            } else {
                                mode = mode.clear_popup();
                            }
                            continue;
                        }
                        PopupMessage::GoToSelect { select, should_clear } => {
                            if let Some(editor) = workspace.get_active() {
                                editor.go_to_select(select);
                                if should_clear {
                                    mode = mode.clear_popup();
                                }
                            } else {
                                mode = mode.clear_popup();
                            }
                            continue;
                        }
                        PopupMessage::GoToLine(idx) => {
                            if let Some(editor) = workspace.get_active() {
                                editor.go_to(idx);
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::ReplaceSelect(new, select) => {
                            if let Some(editor) = workspace.get_active() {
                                editor.replace_select(select, new.as_str());
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::UpdateWorkspace => {
                            mode.update_workspace(&mut workspace);
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
                                    workspace.new_from(new_path).await;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::CreateFileOrFolderBase(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder_base(name) {
                                if !new_path.is_dir() {
                                    workspace.new_from(new_path).await;
                                    mode = Mode::Insert;
                                }
                            }
                            mode = mode.clear_popup();
                            continue;
                        }
                        PopupMessage::Rename(new_name) => {
                            mode = mode.clear_popup();
                            workspace.renames(new_name).await;
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
                    GeneralAction::Replace if matches!(mode, Mode::Insert) => {
                        mode = mode.popup(replace_in_editor_popup());
                    }
                    GeneralAction::SelectOpenEditor => {
                        mode = mode.popup(editor_selector(workspace.tabs()));
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
                            workspace.new_from(file_path).await;
                        }
                    }
                    GeneralAction::FinishOrSelect => {
                        if file_tree.on_open_tabs {
                            mode = Mode::Insert;
                        } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            if !file_path.is_dir() {
                                workspace.new_from(file_path).await;
                                mode = Mode::Insert;
                            }
                        }
                    }
                    GeneralAction::NextTab => {
                        if let Some(editor_id) = workspace.state.selected() {
                            file_tree.on_open_tabs = true;
                            if editor_id >= workspace.editors.len() - 1 {
                                workspace.state.select(Some(0))
                            } else {
                                workspace.state.select(Some(editor_id + 1))
                            }
                        }
                    }
                    GeneralAction::Exit => {
                        if workspace.are_updates_saved() && !matches!(mode, Mode::Popup(..)) {
                            break;
                        } else {
                            mode = mode.popup(save_all_popup());
                        }
                    }
                    GeneralAction::FileTreeModeOrCancelInput => mode = Mode::Select,
                    GeneralAction::SaveAll => workspace.save().await,
                    GeneralAction::HideFileTree => file_tree.toggle(),
                    GeneralAction::PreviousTab => {
                        if let Some(editor_id) = workspace.state.selected() {
                            file_tree.on_open_tabs = true;
                            if editor_id == 0 {
                                workspace.state.select(Some(workspace.editors.len() - 1))
                            } else {
                                workspace.state.select(Some(editor_id - 1))
                            }
                        }
                    }
                    GeneralAction::RefreshSettings => {
                        let new_key_map = KeyMap::new();
                        general_key_map = new_key_map.general_key_map();
                        workspace.refresh_cfg(new_key_map.editor_key_map()).await;
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

        events.exchange_footer(&mut footer);
        events.exchange_ws(&mut workspace);
        events.exchange_tree(&mut file_tree);

        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    workspace.graceful_exit().await;
    Ok(())
}
