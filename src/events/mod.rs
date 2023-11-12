mod file_tree_events;
mod footer_events;
pub mod messages;
mod workspace_events;

use std::cell::RefCell;
use std::rc::Rc;

pub use self::file_tree_events::TreeEvent;
pub use self::footer_events::FooterEvent;
pub use self::workspace_events::WorkspaceEvent;
use crate::components::Footer;
use crate::components::Tree;
use crate::components::Workspace;
use crate::configs::Mode;

#[derive(Default)]
pub struct Events {
    pub footer: Vec<FooterEvent>,
    pub workspace: Vec<WorkspaceEvent>,
    pub tree: Vec<TreeEvent>,
}

impl Events {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::default()))
    }

    pub async fn handle_events(
        event: &Rc<RefCell<Self>>,
        tree: &mut Tree,
        workspace: &mut Workspace,
        footer: &mut Footer,
        mode: &mut Mode,
    ) {
        let sync = {
            let mut borrow = event.borrow_mut();
            let mut sync_events = borrow.exchange_tree(tree, mode);
            sync_events.append(&mut borrow.exchange_ws(workspace, mode));
            borrow.exchange_footer(footer);
            sync_events
        };
        for ev in sync {
            ev.async_map(workspace, mode).await;
        }
    }

    pub fn message(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Message(msg.into()));
    }

    pub fn overwrite(&mut self, msg: impl Into<String>) {
        self.footer.push(FooterEvent::Overwrite(msg.into()))
    }

    pub fn exchange_footer(&mut self, footer: &mut Footer) {
        for event in self.footer.drain(..) {
            event.map(footer);
        }
    }

    pub fn exchange_ws(&mut self, workspace: &mut Workspace, mode: &mut Mode) -> Vec<WorkspaceEvent> {
        self.workspace.drain(..).flat_map(|e| e.map_if_sync(workspace, mode)).collect()
    }

    pub fn exchange_tree(&mut self, tree: &mut Tree, mode: &mut Mode) -> Vec<WorkspaceEvent> {
        self.tree.drain(..).flat_map(|e| e.map(tree, mode)).collect()
    }
}
