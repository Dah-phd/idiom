use super::{GlobalState, PopupMessage};
use crate::popups::popup_replace::ReplacePopup;
use crate::popups::popups_editor::selector_ranges;
use crate::popups::{popup_tree_search::ActiveFileSearch, PopupInterface};
use crate::tree::Tree;
use crate::workspace::Workspace;
use crate::{configs::FileType, lsp::Diagnostic, workspace::CursorPosition};
use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};
use lsp_types::{CompletionItem, CompletionTextEdit, InsertTextFormat};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum IdiomEvent {
    PopupAccess,
    Open(PathBuf),
    OpenAtLine(PathBuf, usize),
    OpenAtSelect(PathBuf, (CursorPosition, CursorPosition)),
    SelectPath(PathBuf),
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    RenameFile(String),
    SearchFiles(String),
    RegisterLSP(Arc<Mutex<HashMap<PathBuf, Diagnostic>>>),
    FileUpdated(PathBuf),
    CheckLSP(FileType),
    AutoComplete(String),
    Snippet(String, Option<(usize, usize)>),
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
    GoToSelect {
        select: (CursorPosition, CursorPosition),
        clear_popup: bool,
    },
    GoToLine(usize),
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
            IdiomEvent::SearchFiles(pattern) => {
                if pattern.len() > 1 {
                    let mut new_popup = ActiveFileSearch::new(pattern);
                    new_popup.component_access(ws, tree);
                    gs.popup(new_popup);
                } else {
                    gs.popup(ActiveFileSearch::new(pattern));
                }
            }
            IdiomEvent::Open(path) => {
                tree.select_by_path(&path);
                gs.clear_popup();
                if path.is_dir() {
                    gs.select_mode();
                } else {
                    match ws.new_from(path, gs).await {
                        Ok(..) => gs.insert_mode(),
                        Err(error) => gs.error(error.to_string()),
                    }
                }
            }
            IdiomEvent::OpenAtLine(path, line) => {
                tree.select_by_path(&path);
                gs.clear_popup();
                match ws.new_at_line(path, line, gs).await {
                    Ok(..) => gs.insert_mode(),
                    Err(error) => gs.error(error.to_string()),
                }
            }
            IdiomEvent::OpenAtSelect(path, (from, to)) => {
                tree.select_by_path(&path);
                match ws.new_from(path, gs).await {
                    Ok(..) => {
                        gs.insert_mode();
                        if let Some(editor) = ws.get_active() {
                            editor.go_to_select(from, to);
                            gs.clear_popup();
                        } else {
                            gs.clear_popup();
                        }
                    }
                    Err(error) => gs.error(error.to_string()),
                }
            }
            IdiomEvent::GoToSelect { select: (from, to), clear_popup } => {
                if let Some(editor) = ws.get_active() {
                    editor.go_to_select(from, to);
                    if clear_popup {
                        gs.clear_popup();
                    } else {
                        editor.render(gs);
                    }
                } else {
                    gs.clear_popup();
                }
            }
            IdiomEvent::SelectPath(path) => {
                tree.select_by_path(&path);
            }
            IdiomEvent::CreateFileOrFolder(name) => {
                if let Ok(new_path) = tree.create_file_or_folder(name) {
                    if !new_path.is_dir() {
                        match ws.new_at_line(new_path, 0, gs).await {
                            Ok(..) => {
                                gs.insert_mode();
                                if let Some(editor) = ws.get_active() {
                                    editor.update_status.deny();
                                }
                            }
                            Err(error) => gs.error(error.to_string()),
                        };
                    }
                }
                gs.clear_popup();
            }
            IdiomEvent::CreateFileOrFolderBase(name) => {
                if let Ok(new_path) = tree.create_file_or_folder_base(name) {
                    if !new_path.is_dir() {
                        match ws.new_at_line(new_path, 0, gs).await {
                            Ok(..) => {
                                gs.insert_mode();
                                if let Some(editor) = ws.get_active() {
                                    editor.update_status.deny();
                                }
                            }
                            Err(error) => gs.error(error.to_string()),
                        };
                    }
                }
                gs.clear_popup();
            }
            IdiomEvent::RenameFile(name) => {
                if let Some(result) = tree.rename_path(name) {
                    match result {
                        Ok((old, new_path)) => ws.rename_editors(old, new_path, gs),
                        Err(err) => gs.messages.error(err.to_string()),
                    }
                };
                gs.clear_popup();
            }
            IdiomEvent::RegisterLSP(lsp) => {
                tree.register_lsp(lsp);
            }
            IdiomEvent::AutoComplete(completion) => {
                if let Some(editor) = ws.get_active() {
                    editor.replace_token(completion);
                }
            }
            IdiomEvent::Snippet(snippet, cursor_offset) => {
                if let Some(editor) = ws.get_active() {
                    editor.insert_snippet(snippet, cursor_offset);
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
            IdiomEvent::GoToLine(idx) => {
                if let Some(editor) = ws.get_active() {
                    editor.go_to(idx);
                }
                gs.clear_popup();
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

fn parse_snippet(snippet: String) -> IdiomEvent {
    let mut cursor_offset = None;
    let mut named = false;
    let mut text = String::default();
    let mut is_expr = false;
    let mut line_offset = 0;
    let mut char_offset = 0;
    for ch in snippet.chars() {
        if ch == '\n' {
            line_offset += 1;
            char_offset = 0;
            text.push(ch);
        } else {
            if named {
                if ch == '}' {
                    named = false;
                    continue;
                };
                if ch == ':' || ch.is_numeric() {
                    continue;
                };
            } else if is_expr {
                if ch.is_numeric() {
                    continue;
                };
                if ch == '{' {
                    named = true;
                    cursor_offset = None;
                    continue;
                };
                is_expr = false;
            } else if ch == '$' {
                is_expr = true;
                if cursor_offset.is_none() {
                    cursor_offset.replace((line_offset, char_offset));
                };
                continue;
            };
            char_offset += 1;
            text.push(ch);
        };
    }
    IdiomEvent::Snippet(text, cursor_offset)
}

impl From<IdiomEvent> for PopupMessage {
    fn from(event: IdiomEvent) -> Self {
        PopupMessage::Tree(event)
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

impl From<CompletionItem> for IdiomEvent {
    fn from(item: CompletionItem) -> Self {
        let parser = match item.insert_text_format {
            Some(InsertTextFormat::SNIPPET) => parse_snippet,
            _ => IdiomEvent::AutoComplete,
        };
        if let Some(text) = item.insert_text {
            return (parser)(text);
        }
        if let Some(edit) = item.text_edit {
            match edit {
                CompletionTextEdit::Edit(edit) => {
                    return (parser)(edit.new_text);
                }
                CompletionTextEdit::InsertAndReplace(edit) => {
                    return (parser)(edit.new_text);
                }
            };
        }
        IdiomEvent::AutoComplete(item.label)
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
