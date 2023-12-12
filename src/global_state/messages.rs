use super::{footer_events::FooterEvent, workspace_events::WorkspaceEvent, TreeEvent};

#[derive(Debug, Clone, Default)]
pub enum PopupMessage {
    CreateFileOrFolder(String),
    CreateFileOrFolderBase(String),
    SelectTreeFiles(String),
    SelectTreeFilesFull(String),
    UpdateWorkspace(WorkspaceEvent),
    UpdateFooter(FooterEvent),
    UpdateTree(TreeEvent),
    Exit,
    SaveAndExit,
    Done,
    #[default]
    None,
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
