use tokio::process::Command;

pub fn start_lsp() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("/home/dah/Downloads/extension/server/rust-analyzer");
    cmd
}
