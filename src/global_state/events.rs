use super::PopupMessage;
use crate::lsp::Diagnostic;
use crate::workspace::CursorPosition;
use lsp_types::{request::GotoDeclarationResponse, Location, LocationLink, WorkspaceEdit};
use lsp_types::{CompletionItem, CompletionTextEdit, InsertTextFormat};

use crate::configs::FileType;
use crate::footer::Footer;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

#[derive(Clone)]
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
    RegisterLSP(Arc<Mutex<HashMap<PathBuf, Diagnostic>>>),
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
        clear_popup: bool,
    },
    AutoComplete(String),
    ActivateEditor(usize),
    FindSelector(String),
    FindToReplace(String, Vec<(CursorPosition, CursorPosition)>),
    Open(PathBuf, usize),
    InsertText(String),
    Snippet(String),
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

impl From<CompletionItem> for WorkspaceEvent {
    fn from(item: CompletionItem) -> Self {
        let event_type = match item.insert_text_format {
            Some(InsertTextFormat::SNIPPET) => WorkspaceEvent::Snippet,
            _ => WorkspaceEvent::AutoComplete,
        };
        if let Some(text) = item.insert_text {
            return event_type(text);
        }
        if let Some(edit) = item.text_edit {
            match edit {
                CompletionTextEdit::Edit(edit) => {
                    return event_type(edit.new_text);
                }
                CompletionTextEdit::InsertAndReplace(edit) => {
                    return event_type(edit.new_text);
                }
            };
        }
        event_type(item.label)
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
