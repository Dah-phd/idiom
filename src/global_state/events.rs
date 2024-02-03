use super::PopupMessage;
use crate::workspace::CursorPosition;
use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};

use crate::configs::FileType;
use crate::footer::Footer;
use std::path::PathBuf;

#[allow(dead_code)] // TODO replace normal events
#[derive(Debug, Clone)]
pub enum StateEvent {
    PopupAccess,
    Open(PathBuf),
    OpenAtLine(PathBuf, usize),
    OpenAtSelect(PathBuf, (CursorPosition, CursorPosition)),
    SelectPath(PathBuf),
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    RenameFile(String),
    SearchFiles(String),
    Resize {
        height: u16,
        width: u16,
    },
    Message(String),
    Error(String),
    Success(String),
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
    CheckLSP(FileType),
    WorkspaceEdit(WorkspaceEdit),
    Exit,
    SaveAndExit,
}

#[derive(Debug, Clone)]
pub enum TreeEvent {
    PopupAccess,
    Open(PathBuf),
    OpenAtLine(PathBuf, usize),
    OpenAtSelect(PathBuf, (CursorPosition, CursorPosition)),
    SelectPath(PathBuf),
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    RenameFile(String),
    SearchFiles(String),
}

impl From<TreeEvent> for PopupMessage {
    fn from(event: TreeEvent) -> Self {
        PopupMessage::Tree(event)
    }
}

impl From<Location> for TreeEvent {
    fn from(loc: Location) -> Self {
        Self::OpenAtSelect(PathBuf::from(loc.uri.path()), (loc.range.start.into(), loc.range.end.into()))
    }
}

impl From<LocationLink> for TreeEvent {
    fn from(loc: LocationLink) -> Self {
        Self::OpenAtSelect(
            PathBuf::from(loc.target_uri.path()),
            (loc.target_range.start.into(), loc.target_range.end.into()),
        )
    }
}

#[derive(Debug, Clone)]
pub enum FooterEvent {
    Message(String),
    Error(String),
    Success(String),
}

impl FooterEvent {
    pub fn map(self, footer: &mut Footer) {
        match self {
            Self::Message(message) => footer.message(message),
            Self::Error(message) => footer.error(message),
            Self::Success(message) => footer.success(message),
        }
    }
}

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
    Resize,
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

impl TryFrom<GotoDeclarationResponse> for TreeEvent {
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
