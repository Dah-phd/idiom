use crate::{
    configs::{GeneralAction, KeyMap},
    global_state::GlobalState,
    popups::{
        popup_find::{FindPopup, GoToLinePopup},
        popup_replace::ReplacePopup,
        popup_tree_search::ActivePathSearch,
        popups_editor::{save_all_popup, selector_editors},
    },
    render::backend::Backend,
    runner::EditorTerminal,
    tree::Tree,
    workspace::Workspace,
};
use anyhow::Result;
use crossterm::event::Event;
use std::{path::PathBuf, time::Instant};

pub async fn app(open_file: Option<PathBuf>, backend: Backend) -> Result<()> {
    let mut gs = GlobalState::new(backend)?;
    let configs = gs.unwrap_default_result(KeyMap::new(), ".keys: ");
    let mut last_frame_start = Instant::now();
    let mut general_key_map = configs.general_key_map();

    // COMPONENTS
    let mut tree = Tree::new(configs.tree_key_map());
    let mut workspace = Workspace::new(configs.editor_key_map(), tree.get_base_file_names(), &mut gs).await;
    let mut term = EditorTerminal::new(gs.editor_area.width as u16);

    // CLI SETUP
    if let Some(path) = open_file {
        tree.select_by_path(&path);
        if gs.try_new_editor(&mut workspace, path).await {
            gs.insert_mode();
            workspace.render(&mut gs)?;
        };
    }

    drop(configs);

    loop {
        if let Some(editor) = workspace.get_active() {
            editor.render(&mut gs)?;
        }

        workspace.render(&mut gs)?;

        tree.direct_render(&mut gs)?;

        gs.render_popup_if_exists()?;

        if crossterm::event::poll(last_frame_start.elapsed())? {
            last_frame_start = Instant::now();
            match crossterm::event::read()? {
                Event::Key(key) => {
                    if gs.map_key(&key, &mut workspace, &mut tree, &mut term)? {
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
                            };
                        }
                        GeneralAction::Replace => {
                            if gs.is_insert() {
                                gs.popup(ReplacePopup::new());
                            };
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
                            };
                        }
                        GeneralAction::Exit => {
                            if workspace.are_updates_saved() && gs.popup.is_none() {
                                gs.exit = true;
                            } else {
                                gs.popup(save_all_popup());
                            };
                        }
                        GeneralAction::FileTreeModeOrCancelInput => gs.select_mode(),
                        GeneralAction::SaveAll => workspace.save(&mut gs),
                        GeneralAction::HideFileTree => {
                            gs.toggle_tree();
                        }
                        GeneralAction::RefreshSettings => {
                            let new_key_map = gs.unwrap_default_result(KeyMap::new(), ".keys: ");
                            general_key_map = new_key_map.general_key_map();
                            tree.key_map = new_key_map.tree_key_map();
                            workspace.refresh_cfg(new_key_map.editor_key_map(), &mut gs).await;
                        }
                        GeneralAction::GoToLinePopup => {
                            if gs.is_insert() {
                                gs.popup(GoToLinePopup::new());
                            };
                        }
                        GeneralAction::ToggleTerminal => {
                            gs.toggle_terminal(&mut term);
                        }
                    }
                }
                Event::Resize(width, height) => {
                    gs.full_resize(height, width, &mut workspace);
                    term.resize(gs.editor_area.width as u16);
                }
                Event::Mouse(event) => gs.map_mouse(event, &mut tree, &mut workspace),
                _ => (),
            }
        }

        if gs.exchange_should_exit(&mut tree, &mut workspace).await {
            workspace.graceful_exit().await;
            break;
        };
    }
    Ok(())
}
