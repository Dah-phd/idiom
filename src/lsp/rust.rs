use tokio::process::Command;

pub fn start_lsp() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg("/home/dah/.vscode/extensions/rust-lang.rust-analyzer-0.3.1607-linux-x64/server/rust-analyzer");
    cmd
}
