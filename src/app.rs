use crate::{
    configs::{EditorConfigs, GeneralAction, KeyMap, KEY_MAP},
    error::IdiomResult,
    global_state::{GlobalState, IdiomEvent},
    popups::{
        pallet::Pallet,
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
use std::{io::Write, path::PathBuf, time::Duration};

const MIN_FRAMERATE: Duration = Duration::from_millis(8);

pub async fn app(open_file: Option<PathBuf>, backend: Backend) -> IdiomResult<()> {
    // builtin cursor is not used - cursor is positioned during render

    let mut gs = GlobalState::new(backend)?;
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
                                        if let Some(popup) = FindPopup::new(gs.editor_area, gs.theme.accent_style) {
                                            gs.popup(popup);
                                        }
                                    } else {
                                        gs.popup(ActivePathSearch::new());
                                    };
                                }
                                GeneralAction::Replace => {
                                    if gs.is_insert() {
                                        if let Some(popup) = ReplacePopup::new(gs.editor_area, gs.theme.accent_style) {
                                            gs.popup(popup);
                                        }
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
                                    let (new_general, new_editor_key_map, new_tree_key_map) =
                                        gs.unwrap_or_default(KeyMap::new(), KEY_MAP).unpack();
                                    general_key_map = new_general;
                                    tree.key_map = new_tree_key_map;
                                    workspace.refresh_cfg(new_editor_key_map, &mut gs);
                                }
                                GeneralAction::GoToLinePopup => {
                                    if gs.is_insert() {
                                        if let Some(popup) = workspace.get_active().and_then(|editor| {
                                            let current_line = editor.cursor.line;
                                            GoToLinePopup::new(current_line, gs.editor_area, gs.theme.accent_style)
                                        }) {
                                            gs.popup(popup);
                                        }
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
        gs.backend().flush()?;

        // do event exchanges
        if gs.exchange_should_exit(&mut tree, &mut workspace).await {
            workspace.graceful_exit().await;
            return Ok(());
        };
    }
}
