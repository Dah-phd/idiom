use super::PopupMessage;
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
