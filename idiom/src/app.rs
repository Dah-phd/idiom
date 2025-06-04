use crate::{
    configs::GeneralAction,
    embeded_term::EditorTerminal,
    error::IdiomResult,
    global_state::{CrossTerm, GlobalState, IdiomEvent},
    popups::{
        get_init_screen, get_new_screen_size,
        pallet::Pallet,
        popup_find::{FindPopup, GoToLinePopup},
        popup_replace::ReplacePopup,
        popup_tree_search::ActivePathSearch,
        popups_editor::selector_editors,
        should_save_and_exit, Popup,
    },
    tree::Tree,
    workspace::Workspace,
};
use crossterm::event::Event;
use std::{path::PathBuf, time::Duration};

pub const MIN_FRAMERATE: Duration = Duration::from_millis(8);
pub const MIN_HEIGHT: u16 = 6;
pub const MIN_WIDTH: u16 = 40;

pub async fn app(open_file: Option<PathBuf>, mut backend: CrossTerm) -> IdiomResult<()> {
    // builtin cursor is not used - cursor is positioned during render

    let screen_rect = get_init_screen(&mut backend)?;
    let mut gs = GlobalState::new(screen_rect, backend);
    let (mut general_key_map, editor_key_map, tree_key_map) = gs.get_key_maps();
    let mut base_configs = gs.get_configs();
    let integrated_shell = base_configs.shell.take();

    // INIT COMPONENTS
    let mut tree = Tree::new(tree_key_map, &mut gs);
    let mut term = EditorTerminal::new(integrated_shell);
    let lsp_servers = base_configs.init_preloaded_lsp_servers(tree.get_base_file_names(), &mut gs).await;
    let mut workspace = Workspace::new(editor_key_map, base_configs, lsp_servers).await;

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
                                        ActivePathSearch::run(&mut gs, &mut workspace, &mut tree, &mut term);
                                    };
                                }
                                GeneralAction::Replace => {
                                    if gs.is_insert() {
                                        ReplacePopup::run_inplace(&mut gs, &mut workspace, &mut tree, &mut term);
                                    };
                                }
                                GeneralAction::SelectOpenEditor => {
                                    let tabs = workspace.tabs();
                                    match tabs.len() {
                                        0 => (),
                                        1 => gs.insert_mode(),
                                        _ => {
                                            let mut selector = selector_editors(tabs);
                                            let result = selector.run(&mut gs, &mut workspace, &mut tree, &mut term);
                                            gs.log_if_error(result);
                                        }
                                    }
                                }
                                GeneralAction::GoToTabs => {
                                    if !workspace.is_empty() {
                                        workspace.toggle_tabs();
                                        gs.insert_mode();
                                    };
                                }
                                GeneralAction::InvokePallet => {
                                    Pallet::run(&mut gs, &mut workspace, &mut tree, &mut term);
                                }
                                GeneralAction::Exit => {
                                    if workspace.are_updates_saved(&mut gs)
                                        || should_save_and_exit(&mut gs, &mut workspace, &mut tree, &mut term)
                                    {
                                        return Ok(());
                                    };
                                }
                                GeneralAction::FileTreeModeOrCancelInput => gs.select_mode(),
                                GeneralAction::SaveAll => workspace.save_all(&mut gs),
                                GeneralAction::HideFileTree => {
                                    gs.toggle_tree();
                                }
                                GeneralAction::RefreshSettings => {
                                    let (new_general, new_editor_key_map, new_tree_key_map) = gs.get_key_maps();
                                    general_key_map = new_general;
                                    tree.key_map = new_tree_key_map;
                                    let base_configs = workspace.refresh_cfg(new_editor_key_map, &mut gs);
                                    let integrated_shell = base_configs.shell.take();
                                    gs.git_tui = base_configs.git_tui.take();
                                    term.set_shell(integrated_shell);
                                }
                                GeneralAction::GoToLine => {
                                    if gs.is_insert() {
                                        GoToLinePopup::run_inplace(&mut gs, &mut workspace, &mut tree, &mut term);
                                    };
                                }
                                GeneralAction::GitTui => {
                                    gs.event.push(IdiomEvent::EmbededApp(gs.git_tui.to_owned()));
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
                        let (new_width, new_height) = get_new_screen_size(gs.backend())?;
                        width = new_width;
                        height = new_height;
                    }
                    gs.full_resize(height, width);
                    let editor_rect = gs.calc_editor_rect();
                    workspace.resize_all(editor_rect.width, editor_rect.height as usize);
                    term.resize(editor_rect);
                }
                Event::Mouse(event) => gs.map_mouse(event, &mut tree, &mut workspace, &mut term),
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
