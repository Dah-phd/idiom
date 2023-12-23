use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};

use crate::{configs::FileType, workspace::CursorPosition};
use std::path::PathBuf;

use super::PopupMessage;

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    PopupAccess,
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
    Open(PathBuf, usize),
    CheckLSP(FileType),
    WorkspaceEdit(WorkspaceEdit),
    Exit,
    SaveAndExit,
}

impl From<WorkspaceEvent> for PopupMessage {
    fn from(event: WorkspaceEvent) -> Self {
        PopupMessage::Workspace(event)
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
