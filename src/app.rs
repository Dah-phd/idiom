use crate::{
    configs::{GeneralAction, KeyMap},
    footer::Footer,
    global_state::{GlobalState, Mode},
    popups::{
        popup_find::FindPopup,
        popup_replace::ReplacePopup,
        popup_tree_search::ActiveTreeSearch,
        popups_editor::{go_to_line_popup, save_all_popup, selector_editors},
        popups_tree::{create_file_popup, rename_file_popup},
    },
    terminal::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
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

pub async fn app(mut terminal: Terminal<CrosstermBackend<Stdout>>, open_file: Option<PathBuf>) -> Result<()> {
    let configs = KeyMap::new();
    let mut clock = Instant::now();
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
        if gs.try_new_editor(&mut workspace, path).await {
            gs.mode = Mode::Insert;
        };
    }

    drop(configs);

    loop {
        if matches!(gs.mode, Mode::Select) {
            let _ = terminal.hide_cursor();
        }
        terminal.draw(|frame| {
            let mut screen = frame.size();
            screen = file_tree.render_with_remainder(frame, screen);
            screen = footer.render_with_remainder(frame, screen, gs.mode_span(), workspace.get_stats());
            screen = tmux.render_with_remainder(frame, screen);
            workspace.render(frame, screen, &mut gs);
            gs.render_popup_if_exists(frame);
        })?;

        let timeout = TICK.checked_sub(clock.elapsed()).unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if !tmux.active && gs.map_modal_if_exists(&key) || workspace.map(&key, &mut gs) {
                    continue;
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
                        if matches!(gs.mode, Mode::Insert) {
                            gs.popup(FindPopup::new());
                        } else {
                            gs.popup(ActiveTreeSearch::new());
                        }
                    }
                    GeneralAction::Replace if matches!(gs.mode, Mode::Insert) => {
                        gs.popup(ReplacePopup::new());
                    }
                    GeneralAction::SelectOpenEditor => {
                        gs.popup(selector_editors(workspace.tabs()));
                    }
                    GeneralAction::NewFile => {
                        gs.popup(create_file_popup(file_tree.get_first_selected_folder_display()));
                    }
                    GeneralAction::Rename => {
                        if matches!(gs.mode, Mode::Select) {
                            if let Some(tree_path) = file_tree.get_selected() {
                                gs.popup(rename_file_popup(tree_path.path().display().to_string()));
                            }
                        }
                    }
                    GeneralAction::Expand => {
                        if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            gs.try_new_editor(&mut workspace, file_path).await;
                        }
                    }
                    GeneralAction::FinishOrSelect => {
                        if file_tree.on_open_tabs {
                            gs.mode = Mode::Insert;
                        } else if let Some(file_path) = file_tree.expand_dir_or_get_path() {
                            if !file_path.is_dir() && gs.try_new_editor(&mut workspace, file_path).await {
                                gs.mode = Mode::Insert;
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
                        if workspace.are_updates_saved() && gs.popup.is_none() {
                            gs.exit = true;
                        } else {
                            gs.popup(save_all_popup());
                        }
                    }
                    GeneralAction::FileTreeModeOrCancelInput => gs.mode = Mode::Select,
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
                    GeneralAction::GoToLinePopup if matches!(gs.mode, Mode::Insert) => {
                        gs.popup(go_to_line_popup());
                    }
                    GeneralAction::ToggleTerminal => {
                        tmux.toggle();
                    }
                    _ => (),
                }
            }
        }

        if gs.exchange_should_exit(&mut file_tree, &mut workspace, &mut footer).await {
            workspace.graceful_exit().await;
            break;
        };

        if clock.elapsed() >= TICK {
            clock = Instant::now();
        }
    }
    Ok(())
}
