use lsp_types::InitializeParams;
use tokio::process::Command;

pub fn start_lsp() -> Command {
    let params: InitializeParams = InitializeParams {
        process_id: Some(std::process::id()),
        ..Default::default()
    };
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg("/home/dah/Downloads/extension/server/rust-analyzer");
    cmd
}
