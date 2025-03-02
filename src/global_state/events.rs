use super::{GlobalState, PopupMessage};
use crate::lsp::TreeDiagnostics;
use crate::popups::{
    popup_replace::ReplacePopup, popup_tree_search::ActiveFileSearch, popups_editor::selector_ranges, PopupInterface,
};
use crate::tree::Tree;
use crate::workspace::line::EditorLine;
use crate::workspace::{add_editor_from_data, Workspace};
use crate::{configs::FileType, workspace::CursorPosition};
use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub enum IdiomEvent {
    PopupAccess,
    PopupAccessOnce,
    NewPopup(fn() -> Box<dyn PopupInterface>),
    OpenAtLine(PathBuf, usize),
    OpenAtSelect(PathBuf, (CursorPosition, CursorPosition)),
    OpenLSPErrors,
    SelectPath(PathBuf),
    CreateFileOrFolder {
        name: String,
        from_base: bool,
    },
    RenameFile(String),
    RenamedFile {
        from_path: PathBuf,
        to_path: PathBuf,
    },
    SearchFiles(String),
    FileUpdated(PathBuf),
    CheckLSP(FileType),
    TreeDiagnostics(TreeDiagnostics),
    AutoComplete(String),
    Snippet {
        snippet: String,
        cursor_offset: Option<(usize, usize)>,
        relative_select: Option<((usize, usize), usize)>,
    },
    InsertText(String),
    WorkspaceEdit(WorkspaceEdit),
    FindSelector(String),
    ActivateEditor(usize),
    ReplaceAll(String, Vec<(CursorPosition, CursorPosition)>),
    FindToReplace(String, Vec<(CursorPosition, CursorPosition)>),
    ReplaceNextSelect {
        new_text: String,
        select: (CursorPosition, CursorPosition),
        next_select: Option<(CursorPosition, CursorPosition)>,
    },
    GoToLine {
        line: usize,
        clear_popup: bool,
    },
    GoToSelect {
        select: (CursorPosition, CursorPosition),
        clear_popup: bool,
    },
    Resize,
    Save,
    Rebase,
    Exit,
    SaveAndExit,
}

impl IdiomEvent {
    pub async fn handle(self, gs: &mut GlobalState, ws: &mut Workspace, tree: &mut Tree) {
        match self {
            IdiomEvent::PopupAccess => {
                gs.popup.component_access(ws, tree);
            }
            IdiomEvent::PopupAccessOnce => {
                gs.popup.component_access(ws, tree);
                gs.clear_popup();
            }
            IdiomEvent::NewPopup(builder) => {
                gs.clear_popup();
                gs.popup(builder());
            }
            IdiomEvent::SearchFiles(pattern) => {
                if pattern.len() > 1 {
                    let mut new_popup = ActiveFileSearch::new(pattern);
                    new_popup.component_access(ws, tree);
                    gs.popup(new_popup);
                } else {
                    gs.popup(ActiveFileSearch::new(pattern));
                }
            }
            IdiomEvent::OpenAtLine(path, line) => {
                let select_result = tree.select_by_path(&path);
                gs.log_if_error(select_result);
                gs.clear_popup();
                match ws.new_at_line(path, line, gs).await {
                    Ok(..) => gs.insert_mode(),
                    Err(error) => gs.error(error),
                }
            }
            IdiomEvent::OpenAtSelect(path, (from, to)) => {
                let select_result = tree.select_by_path(&path);
                gs.clear_popup();
                gs.log_if_error(select_result);
                match ws.new_from(path, gs).await {
                    Ok(..) => {
                        gs.insert_mode();
                        if let Some(editor) = ws.get_active() {
                            editor.go_to_select(from, to);
                        };
                    }
                    Err(error) => gs.error(error),
                };
            }
            IdiomEvent::OpenLSPErrors => {
                gs.clear_popup();
                match PathBuf::from("./").canonicalize() {
                    Ok(base_path) => {
                        let mut path = base_path.clone();
                        path.push("editor_error.log");
                        let mut id = 0_usize;
                        while path.exists() {
                            path = base_path.clone();
                            path.push(&format!("editor_error_{id}.log"));
                            id += 1;
                        }
                        let file_type = FileType::Ignored;
                        let content: Vec<EditorLine> =
                            gs.messages.get_logs().map(ToOwned::to_owned).map(EditorLine::from).collect();
                        if !content.is_empty() {
                            add_editor_from_data(ws, path, content, file_type, gs);
                        } else {
                            gs.success(" >> no error logs found!");
                        }
                    }
                    Err(error) => gs.error(error),
                }
            }
            IdiomEvent::GoToLine { line, clear_popup } => match ws.get_active() {
                Some(editor) => {
                    editor.go_to(line);
                    match clear_popup {
                        true => gs.clear_popup(),
                        false => {
                            editor.render(gs);
                            gs.popup.mark_as_updated();
                        }
                    }
                }
                None => gs.clear_popup(),
            },
            IdiomEvent::GoToSelect { select: (from, to), clear_popup } => match ws.get_active() {
                Some(editor) => {
                    editor.go_to_select(from, to);
                    match clear_popup {
                        true => gs.clear_popup(),
                        false => {
                            editor.render(gs);
                            gs.popup.mark_as_updated();
                        }
                    }
                }
                None => gs.clear_popup(),
            },
            IdiomEvent::SelectPath(path) => {
                let result = tree.select_by_path(&path);
                gs.log_if_error(result);
            }
            IdiomEvent::TreeDiagnostics(new) => {
                tree.push_diagnostics(new);
            }
            IdiomEvent::CreateFileOrFolder { name, from_base } => {
                if name.is_empty() {
                    gs.error("File creation requires input!");
                } else {
                    let result = match from_base {
                        true => tree.create_file_or_folder_base(name),
                        false => tree.create_file_or_folder(name),
                    };
                    match result {
                        Ok(new_path) => {
                            tree.sync(gs);
                            if !new_path.is_dir() {
                                match ws.new_at_line(new_path.clone(), 0, gs).await {
                                    Ok(..) => {
                                        gs.insert_mode();
                                        if let Some(editor) = ws.get_active() {
                                            editor.update_status.deny();
                                        }
                                    }
                                    Err(error) => gs.error(error),
                                };
                            }
                            tree.sync(gs);
                            let result = tree.select_by_path(&new_path);
                            gs.log_if_error(result);
                        }
                        Err(error) => gs.error(error),
                    }
                }
                gs.clear_popup();
            }
            IdiomEvent::RenameFile(name) => {
                if name.is_empty() {
                    gs.error("Rename requires input!");
                } else if let Some(result) = tree.rename_path(name) {
                    match result {
                        Ok((from_path, to_path)) => ws.rename_editors(from_path, to_path, gs),
                        Err(error) => gs.error(error),
                    }
                };
                gs.clear_popup();
            }
            IdiomEvent::RenamedFile { from_path, to_path } => {
                ws.rename_editors(from_path, to_path, gs);
            }
            IdiomEvent::AutoComplete(completion) => {
                if let Some(editor) = ws.get_active() {
                    editor.replace_token(completion);
                }
            }
            IdiomEvent::Snippet { snippet, cursor_offset, relative_select } => {
                if let Some(editor) = ws.get_active() {
                    match relative_select {
                        Some((cursor_offset, len)) => {
                            editor.insert_snippet_with_select(snippet, cursor_offset, len);
                        }
                        None => editor.insert_snippet(snippet, cursor_offset),
                    }
                };
            }
            IdiomEvent::WorkspaceEdit(edits) => ws.apply_edits(edits, gs),
            IdiomEvent::Resize => {
                ws.resize_all(gs.editor_area.width, gs.editor_area.height as usize);
            }
            IdiomEvent::Rebase => {
                if let Some(editor) = ws.get_active() {
                    editor.rebase(gs);
                }
                gs.clear_popup();
            }
            IdiomEvent::Save => {
                if let Some(editor) = ws.get_active() {
                    editor.save(gs);
                }
                gs.clear_popup();
            }
            IdiomEvent::CheckLSP(ft) => {
                ws.check_lsp(ft, gs).await;
            }
            IdiomEvent::SaveAndExit => {
                ws.save_all(gs);
                gs.exit = true;
            }
            IdiomEvent::Exit => {
                gs.exit = true;
            }
            IdiomEvent::FileUpdated(path) => {
                ws.notify_update(path, gs);
            }
            IdiomEvent::InsertText(insert) => {
                if let Some(editor) = ws.get_active() {
                    editor.insert_text_with_relative_offset(insert);
                };
            }
            IdiomEvent::FindSelector(pattern) => {
                if let Some(editor) = ws.get_active() {
                    gs.insert_mode();
                    gs.popup(selector_ranges(editor.find_with_line(&pattern)));
                } else {
                    gs.clear_popup();
                }
            }
            IdiomEvent::ActivateEditor(idx) => {
                ws.activate_editor(idx, gs);
                gs.clear_popup();
                gs.insert_mode();
            }
            IdiomEvent::FindToReplace(pattern, options) => {
                gs.popup(ReplacePopup::from_search(pattern, options));
            }
            IdiomEvent::ReplaceAll(clip, ranges) => {
                if let Some(editor) = ws.get_active() {
                    editor.mass_replace(ranges, clip);
                }
                gs.clear_popup();
            }
            IdiomEvent::ReplaceNextSelect { new_text, select: (from, to), next_select } => {
                if let Some(editor) = ws.get_active() {
                    editor.replace_select(from, to, new_text.as_str());
                    if let Some((from, to)) = next_select {
                        editor.go_to_select(from, to);
                        editor.render(gs);
                    }
                }
            }
        }
    }
}

impl From<IdiomEvent> for PopupMessage {
    fn from(event: IdiomEvent) -> Self {
        PopupMessage::Event(event)
    }
}

impl From<Location> for IdiomEvent {
    fn from(loc: Location) -> Self {
        Self::OpenAtSelect(PathBuf::from(loc.uri.path().as_str()), (loc.range.start.into(), loc.range.end.into()))
    }
}

impl From<LocationLink> for IdiomEvent {
    fn from(loc: LocationLink) -> Self {
        Self::OpenAtSelect(
            PathBuf::from(loc.target_uri.path().as_str()),
            (loc.target_range.start.into(), loc.target_range.end.into()),
        )
    }
}

impl From<WorkspaceEdit> for IdiomEvent {
    fn from(value: WorkspaceEdit) -> Self {
        Self::WorkspaceEdit(value)
    }
}

impl TryFrom<GotoDeclarationResponse> for IdiomEvent {
    type Error = ();
    fn try_from(value: GotoDeclarationResponse) -> Result<Self, ()> {
        Ok(match value {
            GotoDeclarationResponse::Scalar(location) => location.into(),
            GotoDeclarationResponse::Array(mut arr) => {
                if arr.is_empty() {
                    return Err(());
                }
                arr.remove(0).into()
            }
            GotoDeclarationResponse::Link(mut links) => {
                if links.is_empty() {
                    return Err(());
                }
                links.remove(0).into()
            }
        })
    }
}
