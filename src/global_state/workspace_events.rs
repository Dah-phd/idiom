use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};

use crate::{
    configs::{FileType, Mode},
    popups::{popup_replace::ReplacePopup, popups_editor::selector_ranges},
    workspace::{CursorPosition, Workspace},
};
use std::path::PathBuf;

use super::GlobalState;

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    PopupAccess,
    ReplaceSelect(String, (CursorPosition, CursorPosition)),
    ReplaceNextSelect {
        new_text: String,
        select: (CursorPosition, CursorPosition),
        next_select: Option<(CursorPosition, CursorPosition)>,
    },
    ReplaceAll(String, Vec<(CursorPosition, CursorPosition)>),
    GoToLine(usize),
    GoToSelect {
        select: (CursorPosition, CursorPosition),
        should_clear: bool,
    },
    AutoComplete(String),
    ActivateEditor(usize),
    FindSelector(String),
    FindToReplace(String, Vec<(CursorPosition, CursorPosition)>),
    SelectTreeFiles(String),
    Open(PathBuf, usize),
    CheckLSP(FileType),
    WorkspaceEdit(WorkspaceEdit),
}

impl WorkspaceEvent {
    pub async fn map_if_sync(self, workspace: &mut Workspace, mode: &mut Mode, gs: &mut GlobalState) {
        match self {
            Self::GoToLine(idx) => {
                if let Some(editor) = workspace.get_active() {
                    editor.go_to(idx);
                }
                mode.clear_popup();
            }
            Self::PopupAccess => mode.update_workspace(workspace),
            Self::ReplaceSelect(new, (from, to)) => {
                if let Some(editor) = workspace.get_active() {
                    editor.replace_select(from, to, new.as_str());
                }
                mode.clear_popup();
            }
            Self::ReplaceNextSelect { new_text, select: (from, to), next_select } => {
                if let Some(editor) = workspace.get_active() {
                    editor.replace_select(from, to, new_text.as_str());
                    if let Some((from, to)) = next_select {
                        editor.go_to_select(from, to);
                    }
                }
            }
            Self::ReplaceAll(clip, ranges) => {
                if let Some(editor) = workspace.get_active() {
                    editor.mass_replace(ranges, clip);
                }
                mode.clear_popup();
            }
            Self::GoToSelect { select: (from, to), should_clear } => {
                if let Some(editor) = workspace.get_active() {
                    editor.go_to_select(from, to);
                    if should_clear {
                        mode.clear_popup();
                    }
                } else {
                    mode.clear_popup();
                }
            }
            Self::ActivateEditor(idx) => {
                workspace.state.select(Some(idx));
                mode.clear_popup();
            }
            Self::FindSelector(pattern) => {
                if let Some(editor) = workspace.get_active() {
                    mode.popup_insert(selector_ranges(editor.find_with_line(&pattern)));
                } else {
                    mode.clear_popup();
                }
            }
            Self::FindToReplace(pattern, options) => {
                mode.clear_popup();
                mode.popup(ReplacePopup::from_search(pattern, options));
            }
            Self::AutoComplete(completion) => {
                if let Some(editor) = workspace.get_active() {
                    editor.replace_token(completion);
                }
            }
            Self::WorkspaceEdit(edits) => workspace.apply_edits(edits, gs),
            Self::Open(path, line) => {
                if !path.is_dir() && workspace.new_at_line(path, line, gs).await.is_ok() {
                    *mode = Mode::Insert;
                } else {
                    *mode = Mode::Select;
                }
            }
            Self::CheckLSP(ft) => {
                workspace.check_lsp(ft, gs).await;
            }
            _ => (),
        }
    }

    pub async fn async_map(self, workspace: &mut Workspace, mode: &mut Mode, gs: &mut GlobalState) {
        match self {
            Self::Open(path, line) => {
                if !path.is_dir() && workspace.new_at_line(path, line, gs).await.is_ok() {
                    *mode = Mode::Insert;
                } else {
                    *mode = Mode::Select;
                }
            }
            Self::CheckLSP(ft) => {
                workspace.check_lsp(ft, gs).await;
            }
            _ => (),
        }
    }
}

impl From<WorkspaceEdit> for WorkspaceEvent {
    fn from(value: WorkspaceEdit) -> Self {
        Self::WorkspaceEdit(value)
    }
}

impl From<Location> for WorkspaceEvent {
    fn from(value: Location) -> Self {
        Self::Open(PathBuf::from(value.uri.path()), value.range.start.line as usize)
    }
}

impl From<LocationLink> for WorkspaceEvent {
    fn from(value: LocationLink) -> Self {
        Self::Open(PathBuf::from(value.target_uri.path()), value.target_range.start.line as usize)
    }
}

impl TryFrom<GotoDeclarationResponse> for WorkspaceEvent {
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
