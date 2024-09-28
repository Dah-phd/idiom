use crate::{
    configs::{GeneralAction, KeyMap},
    error::IdiomResult,
    global_state::{GlobalState, IdiomEvent},
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
use crossterm::event::Event;
use std::{path::PathBuf, time::Duration};

const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub async fn app(open_file: Option<PathBuf>, backend: Backend) -> IdiomResult<()> {
    // builtin cursor is not used - cursor is positioned during render

    let mut gs = GlobalState::new(backend)?;
    let configs = gs.unwrap_or_default(KeyMap::new(), ".keys: ");
    let mut general_key_map = configs.general_key_map();

    // COMPONENTS
    let mut tree = Tree::new(configs.tree_key_map());
    let mut workspace = Workspace::new(configs.editor_key_map(), tree.get_base_file_names(), &mut gs).await;
    let mut term = EditorTerminal::new(gs.editor_area.width as u16);

    // CLI SETUP
    if let Some(path) = open_file {
        tree.select_by_path(&path);
        gs.event.push(IdiomEvent::Open(path));
        gs.toggle_tree();
    }

    drop(configs);

    loop {
        // handle input events
        if crossterm::event::poll(MIN_FRAMERATE)? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    if !gs.map_key(&key, &mut workspace, &mut tree, &mut term) {
                        if let Some(action) = general_key_map.map(&key) {
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
                                    if !workspace.is_empty() {
                                        workspace.toggle_tabs();
                                        gs.insert_mode();
                                    };
                                }
                                GeneralAction::Exit => {
                                    if workspace.are_updates_saved() && !gs.has_popup() {
                                        gs.exit = true;
                                    } else {
                                        gs.popup(save_all_popup());
                                    };
                                }
                                GeneralAction::FileTreeModeOrCancelInput => gs.select_mode(),
                                GeneralAction::SaveAll => workspace.save_all(&mut gs),
                                GeneralAction::HideFileTree => {
                                    gs.toggle_tree();
                                }
                                GeneralAction::RefreshSettings => {
                                    let new_key_map = gs.unwrap_or_default(KeyMap::new(), ".keys: ");
                                    general_key_map = new_key_map.general_key_map();
                                    tree.key_map = new_key_map.tree_key_map();
                                    workspace.refresh_cfg(new_key_map.editor_key_map(), &mut gs);
                                }
                                GeneralAction::GoToLinePopup => {
                                    if gs.is_insert() {
                                        gs.popup(GoToLinePopup::new());
                                    };
                                }
                                GeneralAction::ToggleTerminal => {
                                    gs.toggle_terminal(&mut term);
                                }
                                GeneralAction::GoToTab1 => workspace.go_to_tab(0, &mut gs),
                                GeneralAction::GoToTab2 => workspace.go_to_tab(1, &mut gs),
                                GeneralAction::GoToTab3 => workspace.go_to_tab(2, &mut gs),
                                GeneralAction::GoToTab4 => workspace.go_to_tab(3, &mut gs),
                                GeneralAction::GoToTab5 => workspace.go_to_tab(4, &mut gs),
                                GeneralAction::GoToTab6 => workspace.go_to_tab(5, &mut gs),
                                GeneralAction::GoToTab7 => workspace.go_to_tab(6, &mut gs),
                                GeneralAction::GoToTab8 => workspace.go_to_tab(7, &mut gs),
                                GeneralAction::GoToTab9 => workspace.go_to_tab(8, &mut gs),
                            }
                        };
                    }
                }
                Event::Resize(width, height) => {
                    gs.full_resize(height, width);
                    term.resize(gs.editor_area.width as u16);
                }
                Event::Mouse(event) => gs.map_mouse(event, &mut tree, &mut workspace),
                _ => (),
            }
        }

        // render updates
        gs.draw(&mut workspace, &mut tree, &mut term)?;

        // do event exchanges
        if gs.exchange_should_exit(&mut tree, &mut workspace).await {
            workspace.graceful_exit().await;
            break;
        };
    }
    Ok(())
}
