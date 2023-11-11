mod file_tree_events;
mod footer_events;
pub mod messages;
mod workspace_events;

use std::cell::RefCell;
use std::rc::Rc;

use self::file_tree_events::TreeEvent;
use self::footer_events::FooterEvent;
use self::workspace_events::WorkspaceEvent;
use crate::components::Footer;
use crate::components::Tree;
use crate::components::Workspace;

#[derive(Default)]
pub struct Events {
    footer: Vec<FooterEvent>,
    workspace: Vec<WorkspaceEvent>,
    tree: Vec<TreeEvent>,
}

impl Events {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::default()))
    }

    pub fn exchange_footer(&mut self, footer: &mut Footer) {
        for event in self.footer.drain(..) {
            event.map(footer);
        }
    }

    pub fn exchange_ws(&mut self, workspace: &mut Workspace) {
        for event in self.workspace.drain(..) {
            event.map(workspace);
        }
    }

    pub fn exchange_tree(&mut self, tree: &mut Tree) {
        for event in self.tree.drain(..) {
            event.map(tree);
        }
    }
}
