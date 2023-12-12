use crate::{
    components::{
        popups::{
            popup_find::FindPopup,
            popup_replace::ReplacePopup,
            popups_editor::{go_to_line_popup, save_all_popup, selector_editors},
            popups_tree::{create_file_popup, find_in_tree_popup, rename_file_popup, tree_file_selector},
        },
        EditorTerminal, Footer, Tree, Workspace,
    },
    configs::{GeneralAction, KeyMap, Mode},
    global_state::messages::PopupMessage,
    global_state::GlobalState,
};

use anyhow::Result;
use crossterm::event::Event;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::Stdout,
    path::PathBuf,
    time::{Duration, Instant},
};

const TICK: Duration = Duration::from_millis(15);

pub async fn app(terminal: &mut Terminal<CrosstermBackend<&Stdout>>, open_file: Option<PathBuf>) -> Result<()> {
    let configs = KeyMap::new();
    let mut clock = Instant::now();
    let mut mode = Mode::Select;
    let mut general_key_map = configs.general_key_map();
    let mut gs = GlobalState::default();

    // COMPONENTS
    let mut file_tree = Tree::new(open_file.is_none());
    let mut workspace = Workspace::new(configs.editor_key_map());
    let mut tmux = EditorTerminal::new();
    let mut footer = Footer::default();

    // CLI SETUP
    if let Some(path) = open_file {
        file_tree.select_by_path(&path);
        if footer.logged_ok(workspace.new_from(path, &mut gs).await).is_some() {
            mode = Mode::Insert;
        };
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
            workspace.render(frame, screen, &mut gs);
            mode.render_popup_if_exists(frame);
        })?;

        workspace.lexer_updates(&mut gs).await;

        let timeout = TICK.checked_sub(clock.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if !tmux.active && workspace.map(&key, &mut mode, &mut gs) {
                    continue;
                }
                if let Some(msg) = mode.popup_map(&key) {
                    match msg {
                        PopupMessage::Exit => break,
                        PopupMessage::SaveAndExit => {
                            workspace.save_all(&mut gs);
                            break;
                        }
                        PopupMessage::SelectTreeFiles(pattern) => {
                            mode.popup_select(tree_file_selector(file_tree.search_select_files(pattern).await));
                            continue;
                        }
                        PopupMessage::SelectTreeFilesFull(pattern) => {
                            mode.popup_select(tree_file_selector(file_tree.search_files(pattern).await));
                            continue;
                        }
                        PopupMessage::CreateFileOrFolder(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder(name) {
                                if !new_path.is_dir()
                                    && footer.logged_ok(workspace.new_from(new_path, &mut gs).await).is_some()
                                {
                                    mode = Mode::Insert;
                                }
                            }
                            mode.clear_popup();
                            continue;
                        }
                        PopupMessage::CreateFileOrFolderBase(name) => {
                            if let Ok(new_path) = file_tree.create_file_or_folder_base(name) {
                                if !new_path.is_dir()
                                    && footer.logged_ok(workspace.new_from(new_path, &mut gs).await).is_some()
                                {
                                    mode = Mode::Insert;
                                }
                            }
                            mode.clear_popup();
                            continue;
                        }
                        PopupMessage::UpdateWorkspace(event) => {
                            gs.workspace.push(event);
                            continue;
                        }
                        PopupMessage::UpdateFooter(event) => {
                            gs.footer.push(event);
                            continue;
                        }
                        PopupMessage::UpdateTree(event) => {
                            gs.tree.push(event);
                            continue;
                        }
                        PopupMessage::None => continue,
                        PopupMessage::Done => {
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
                    GeneralAction::Find => {
                        if matches!(mode, Mode::Insert) {
                            mode.popup(FindPopup::new());
                        } else {
                            mode.popup(find_in_tree_popup());
                        }
                    }
                    GeneralAction::Replace if matches!(mode, Mode::Insert) => {
                        mode.popup(ReplacePopup::new());
                    }
                    GeneralAction::SelectOpenEditor => {
                        mode.popup(selector_editors(workspace.tabs()));
                    }
                    GeneralAction::NewFile => {
                        mode.popup(create_file_popup(file_tree.get_first_selected_folder_display()));
                    }
                    GeneralAction::Rename => {
                        if matches!(mode, Mode::Select) {
                            if let Some(tree_path) = file_tree.get_selected() {
                                mode.popup(rename_file_popup(tree_path.path().display().to_string()));
                            }
                        }
                    }
                    GeneralAction::Expand => {
                        if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            footer.logged_ok(workspace.new_from(file_path, &mut gs).await);
                        }
                    }
                    GeneralAction::FinishOrSelect => {
                        if file_tree.on_open_tabs {
                            mode = Mode::Insert;
                        } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            if !file_path.is_dir()
                                && footer.logged_ok(workspace.new_from(file_path, &mut gs).await).is_some()
                            {
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
                            mode.popup(save_all_popup());
                        }
                    }
                    GeneralAction::FileTreeModeOrCancelInput => mode = Mode::Select,
                    GeneralAction::SaveAll => workspace.save(&mut gs),
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
                        workspace.refresh_cfg(new_key_map.editor_key_map(), &mut gs).await;
                    }
                    GeneralAction::GoToLinePopup if matches!(mode, Mode::Insert) => {
                        mode.popup(go_to_line_popup());
                    }
                    GeneralAction::ToggleTerminal => {
                        tmux.toggle();
                    }
                    _ => (),
                }
            }
        }

        gs.handle_events(&mut file_tree, &mut workspace, &mut footer, &mut mode).await;

        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    workspace.graceful_exit().await;
    Ok(())
}
