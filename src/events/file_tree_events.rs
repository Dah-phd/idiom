use crate::components::Tree;

pub enum TreeEvent {}

impl TreeEvent {
    pub fn map(self, tree: &mut Tree) {}
}
