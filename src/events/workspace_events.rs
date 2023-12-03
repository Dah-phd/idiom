use lsp_types::{request::GotoDeclarationResponse, GotoDefinitionResponse, Location, LocationLink, WorkspaceEdit};

use crate::{
    components::{popups::editor_popups::select_selector, workspace::CursorPosition, Workspace},
    configs::{FileType, Mode},
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    PopupAccess,
    ReplaceSelect(String, (CursorPosition, CursorPosition)),
    GoToLine(usize),
    GoToSelect { select: (CursorPosition, CursorPosition), should_clear: bool },
    AutoComplete(String),
    ActivateEditor(usize),
    SelectOpenedFile(String),
    SelectTreeFiles(String),
    Open(PathBuf, usize),
    CheckLSP(FileType),
    WorkspaceEdit(WorkspaceEdit),
}

impl WorkspaceEvent {
    pub fn map_if_sync(self, workspace: &mut Workspace, mode: &mut Mode) -> Option<WorkspaceEvent> {
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
            Self::SelectOpenedFile(pattern) => {
                if let Some(editor) = workspace.get_active() {
                    mode.popup_insert(select_selector(editor.find_with_line(&pattern)));
                } else {
                    mode.clear_popup();
                }
            }
            Self::AutoComplete(completion) => {
                if let Some(editor) = workspace.get_active() {
                    editor.replace_token(completion);
                }
            }
            Self::WorkspaceEdit(edits) => workspace.apply_edits(edits),
            _ => return Some(self),
        }
        None
    }

    pub async fn async_map(self, workspace: &mut Workspace, mode: &mut Mode) {
        match self {
            Self::Open(path, line) => {
                if !path.is_dir() {
                    workspace.new_at_line(path, line).await;
                    *mode = Mode::Insert;
                } else {
                    *mode = Mode::Select;
                }
            }
            Self::CheckLSP(ft) => {
                workspace.check_lsp(ft).await;
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

impl From<GotoDeclarationResponse> for WorkspaceEvent {
    fn from(value: GotoDeclarationResponse) -> Self {
        match value {
            GotoDeclarationResponse::Scalar(location) => location.into(),
            GotoDeclarationResponse::Array(mut arr) => arr.remove(0).into(), // ! handle multi select
            GotoDefinitionResponse::Link(mut links) => links.remove(0).into(), // ! handle muti select
        }
    }
}
