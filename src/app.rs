use crate::{
    configs::{EditorConfigs, GeneralAction, KeyMap, KEY_MAP},
    embeded_term::EditorTerminal,
    error::IdiomResult,
    global_state::{GlobalState, IdiomEvent},
    popups::{
        get_init_screen, get_new_screen_size,
        pallet::Pallet,
        popup_find::{FindPopup, GoToLinePopup},
        popup_replace::ReplacePopup,
        popup_tree_search::ActivePathSearch,
        popups_editor::selector_editors,
        save_and_exit_popup,
    },
    render::backend::Backend,
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::Event;
use std::{path::PathBuf, time::Duration};

pub const MIN_FRAMERATE: Duration = Duration::from_millis(8);
pub const MIN_HEIGHT: u16 = 6;
pub const MIN_WIDTH: u16 = 40;

pub async fn app(open_file: Option<PathBuf>, mut backend: Backend) -> IdiomResult<()> {
    // builtin cursor is not used - cursor is positioned during render

    let Some(screen_rect) = get_init_screen(&mut backend) else {
        return Ok(());
    };
    let mut gs = GlobalState::new(screen_rect, backend);
    let (mut general_key_map, editor_key_map, tree_key_map) = gs.unwrap_or_default(KeyMap::new(), KEY_MAP).unpack();
    let mut editor_base_config = gs.unwrap_or_default(EditorConfigs::new(), "editor.toml: ");
    let integrated_shell = editor_base_config.shell.to_owned();

    // INIT COMPONENTS
    let mut tree = Tree::new(tree_key_map, &mut gs);
    let mut term = EditorTerminal::new(integrated_shell);
    let lsp_servers = editor_base_config.init_preloaded_lsp_servers(tree.get_base_file_names(), &mut gs).await;
    let mut workspace = Workspace::new(editor_key_map, editor_base_config, lsp_servers).await;

    // CLI SETUP
    if let Some(path) = open_file {
        tree.select_by_path(&path).unwrap();
        gs.event.push(IdiomEvent::OpenAtLine(path, 0));
        gs.toggle_tree();
    }

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
                                        FindPopup::run_inplace(&mut gs, &mut workspace, &mut tree, &mut term);
                                    } else {
                                        gs.popup(ActivePathSearch::new());
                                    };
                                }
                                GeneralAction::Replace => {
                                    if gs.is_insert() {
                                        ReplacePopup::run_inplace(&mut gs, &mut workspace, &mut tree, &mut term);
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
                                GeneralAction::InvokePallet => gs.popup(Pallet::new(gs.screen_rect)),
                                GeneralAction::Exit => {
                                    if workspace.are_updates_saved(&mut gs) && !gs.has_popup() {
                                        workspace.graceful_exit().await;
                                        return Ok(());
                                    } else if save_and_exit_popup(&mut gs, &mut workspace, &mut tree, &mut term) {
                                        workspace.graceful_exit().await;
                                        return Ok(());
                                    };
                                }
                                GeneralAction::FileTreeModeOrCancelInput => gs.select_mode(),
                                GeneralAction::SaveAll => workspace.save_all(&mut gs),
                                GeneralAction::HideFileTree => {
                                    gs.toggle_tree();
                                }
                                GeneralAction::RefreshSettings => {
                                    let (new_general, new_editor_key_map, new_tree_key_map) =
                                        gs.unwrap_or_default(KeyMap::new(), KEY_MAP).unpack();
                                    general_key_map = new_general;
                                    tree.key_map = new_tree_key_map;
                                    workspace.refresh_cfg(new_editor_key_map, &mut gs);
                                }
                                GeneralAction::GoToLinePopup => {
                                    if gs.is_insert() {
                                        GoToLinePopup::run_inplace(&mut gs, &mut workspace, &mut tree, &mut term);
                                    } else {
                                        gs.event.push(IdiomEvent::EmbededApp("gitui".to_owned()));
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
                Event::Resize(mut width, mut height) => {
                    if width < MIN_WIDTH || height < MIN_HEIGHT {
                        match get_new_screen_size(gs.backend()) {
                            None => {
                                workspace.graceful_exit().await;
                                return Ok(());
                            }
                            Some((new_width, new_height)) => {
                                width = new_width;
                                height = new_height;
                            }
                        }
                    }
                    gs.full_resize(height, width);
                    let editor_rect = gs.calc_editor_rect();
                    workspace.resize_all(editor_rect.width, editor_rect.height as usize);
                    term.resize(gs.editor_area);
                }
                Event::Mouse(event) => gs.map_mouse(event, &mut tree, &mut workspace),
                Event::Paste(clip) => {
                    gs.passthrough_paste(clip, &mut workspace, &mut term);
                }
                _ => (),
            }
        }

        // render updates
        gs.draw(&mut workspace, &mut tree, &mut term);

        // do event exchanges
        gs.handle_events(&mut tree, &mut workspace, &mut term).await
    }
}
