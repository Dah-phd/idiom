use crate::components::Workspace;

pub enum WorkspaceEvent {}

impl WorkspaceEvent {
    pub fn map(self, workspace: &mut Workspace) {}
}
