use crate::{
    components::{popups::editor_popups::select_selector, workspace::Select, Workspace},
    configs::Mode,
};

#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    PopupAccess,
    ReplaceSelect(String, Select),
    GoToLine(usize),
    GoToSelect { select: Select, should_clear: bool },
    AutoComplete(String),
    ActivateEditor(usize),
    SelectOpenedFile(String),
    WorkspaceEdit,
}

impl WorkspaceEvent {
    pub fn map(self, workspace: &mut Workspace, mode: &mut Mode) {
        match self {
            Self::GoToLine(idx) => {
                if let Some(editor) = workspace.get_active() {
                    editor.go_to(idx);
                }
                mode.clear_popup();
            }
            Self::PopupAccess => mode.update_workspace(workspace),
            Self::ReplaceSelect(new, select) => {
                if let Some(editor) = workspace.get_active() {
                    editor.replace_select(select, new.as_str());
                }
                mode.clear_popup();
            }
            Self::GoToSelect { select, should_clear } => {
                if let Some(editor) = workspace.get_active() {
                    editor.go_to_select(select);
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
            Self::AutoComplete(completion) => (),
            Self::WorkspaceEdit => {
                workspace;
            }
        }
    }
}
