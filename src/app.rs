use crate::{
    configs::{GeneralAction, KeyMap},
    footer::Footer,
    global_state::GlobalState,
    popups::{
        popup_find::{FindPopup, GoToLinePopup},
        popup_replace::ReplacePopup,
        popup_tree_search::ActivePathSearch,
        popups_editor::{save_all_popup, selector_editors},
    },
    runner::EditorTerminal,
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
    let size = terminal.size()?;
    let mut gs = GlobalState::new(size.height, size.width);

    // COMPONENTS
    let mut file_tree = Tree::new(configs.tree_key_map());
    let mut workspace = Workspace::new(configs.editor_key_map());
    let mut tmux = EditorTerminal::new();
    let mut footer = Footer::default();
    gs.recalc_editor_size(&file_tree);

    // CLI SETUP
    if let Some(path) = open_file {
        file_tree.select_by_path(&path);
        if gs.try_new_editor(&mut workspace, path).await {
            gs.insert_mode();
        };
    }

    drop(configs);

    loop {
        terminal.draw(|frame| gs.draw(frame, &mut workspace, &mut file_tree, &mut footer, &mut tmux))?;

        let timeout = TICK.saturating_sub(clock.elapsed());

        if crossterm::event::poll(timeout)? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    // order matters
                    if gs.map_key(&key, &mut workspace, &mut file_tree, &mut tmux) {
                        continue;
                    }
                    let action = if let Some(action) = general_key_map.map(&key) {
                        action
                    } else {
                        continue;
                    };
                    match action {
                        GeneralAction::Find => {
                            if gs.is_insert() {
                                gs.popup(FindPopup::new());
                            } else {
                                gs.popup(ActivePathSearch::new());
                            }
                        }
                        GeneralAction::Replace => {
                            if gs.is_insert() {
                                gs.popup(ReplacePopup::new());
                            }
                        }
                        GeneralAction::SelectOpenEditor => {
                            let tabs = workspace.tabs();
                            if !tabs.is_empty() {
                                gs.popup(selector_editors(tabs));
                            };
                        }
                        GeneralAction::GoToTabs => {
                            if !workspace.editors.is_empty() {
                                workspace.toggle_tabs();
                                gs.insert_mode();
                            }
                        }
                        GeneralAction::Exit => {
                            if workspace.are_updates_saved() && gs.popup.is_none() {
                                gs.exit = true;
                            } else {
                                gs.popup(save_all_popup());
                            }
                        }
                        GeneralAction::FileTreeModeOrCancelInput => gs.select_mode(),
                        GeneralAction::SaveAll => workspace.save(&mut gs),
                        GeneralAction::HideFileTree => {
                            gs.toggle_tree();
                            gs.recalc_editor_size(&file_tree);
                        }
                        GeneralAction::RefreshSettings => {
                            let new_key_map = KeyMap::new();
                            general_key_map = new_key_map.general_key_map();
                            file_tree.key_map = new_key_map.tree_key_map();
                            workspace.refresh_cfg(new_key_map.editor_key_map(), &mut gs).await;
                        }
                        GeneralAction::GoToLinePopup => {
                            if gs.is_insert() {
                                gs.popup(GoToLinePopup::new());
                            }
                        }
                        GeneralAction::ToggleTerminal => {
                            gs.toggle_terminal(&mut tmux);
                        }
                    }
                }
                Event::Resize(width, height) => {
                    gs.full_resize(height, width, &file_tree, &mut workspace);
                    tmux.resize(gs.editor_area.width);
                }
                Event::Mouse(event) => gs.map_mouse(event, &mut file_tree, &mut workspace),
                _ => (),
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
