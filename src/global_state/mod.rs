mod clipboard;
mod file_tree_events;
mod footer_events;
pub mod messages;
mod workspace_events;

use std::path::PathBuf;

pub use self::file_tree_events::TreeEvent;
pub use self::footer_events::FooterEvent;
pub use self::workspace_events::WorkspaceEvent;
use crate::components::Footer;
use crate::components::Tree;
use crate::components::Workspace;
use crate::configs::Mode;
use clipboard::Clipboard;

use anyhow::Result;

#[derive(Default)]
pub struct GlobalState {
    pub footer: Vec<FooterEvent>,
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
    pub clipboard: Clipboard,
}

impl GlobalState {
    pub async fn handle_events(
        &mut self,
        tree: &mut Tree,
        workspace: &mut Workspace,
        footer: &mut Footer,
        mode: &mut Mode,
    ) {
        let ws_event = &mut self.exchange_tree(tree, mode);
        self.workspace.append(ws_event);
        self.exchange_ws(workspace, mode).await;
        self.exchange_footer(footer);
    }

    pub fn try_ws_event(&mut self, value: impl TryInto<WorkspaceEvent>) {
        if let Ok(event) = value.try_into() {
            self.workspace.push(event);
        }
    }

    pub fn message(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Message(msg.into()));
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Error(msg.into()));
    }

    pub fn success(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Success(msg.into()));
    }

    pub fn exchange_footer(&mut self, footer: &mut Footer) {
        for event in self.footer.drain(..) {
            event.map(footer);
        }
    }

    pub fn logged_ok<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(val) => Some(val),
            Err(err) => {
                self.error(err.to_string());
                None
            }
        }
    }

    /// Attempts to create new editor if err logs it and returns false else true.
    pub async fn try_new_editor(&mut self, workspace: &mut Workspace, path: PathBuf) -> bool {
        if let Err(err) = workspace.new_from(path, self).await {
            self.error(err.to_string());
            return false;
        }
        true
    }

    pub async fn exchange_ws(&mut self, workspace: &mut Workspace, mode: &mut Mode) {
        let sync: Vec<WorkspaceEvent> = self.workspace.drain(..).collect();
        for event in sync {
            event.map_if_sync(workspace, mode, self).await;
        }
    }

    pub fn exchange_tree(&mut self, tree: &mut Tree, mode: &mut Mode) -> Vec<WorkspaceEvent> {
        self.tree.drain(..).flat_map(|e| e.map(tree, mode)).collect()
    }
}
