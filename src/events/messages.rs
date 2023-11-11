use std::path::PathBuf;

use crate::components::workspace::Select;

use super::{footer_events::FooterEvent, workspace_events::WorkspaceEvent, TreeEvent};

#[derive(Debug, Clone)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    Open(PathBuf, usize),
    SelectTreeFiles(String),
    SelectTreeFilesFull(String),
    Rename(String),
    UpdateWorkspace(WorkspaceEvent),
    UpdateFooter(FooterEvent),
    UpdateTree(TreeEvent),
    Exit,
    SaveAndExit,
    None,
    Done,
}

impl From<TreeEvent> for PopupMessage {
    fn from(value: TreeEvent) -> Self {
        PopupMessage::UpdateTree(value)
    }
}

impl From<WorkspaceEvent> for PopupMessage {
    fn from(value: WorkspaceEvent) -> Self {
        PopupMessage::UpdateWorkspace(value)
    }
}

impl From<FooterEvent> for PopupMessage {
    fn from(value: FooterEvent) -> Self {
        PopupMessage::UpdateFooter(value)
    }
}
