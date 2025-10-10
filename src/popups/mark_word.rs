/// Not a real popup
/// uses similar structure to show marked word
/// but does not implement all APIs
pub struct MarkedWord {}

impl MarkedWord {
    fn run(
        &mut self,
        gs: &mut crate::global_state::GlobalState,
        ws: &mut crate::workspace::Workspace,
        tree: &mut crate::tree::Tree,
        term: &mut crate::embeded_term::EditorTerminal,
    ) -> crate::error::IdiomResult<()> {
        Ok(())
    }
}
