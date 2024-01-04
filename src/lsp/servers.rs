use tokio::process::Command;

pub fn rust_lsp() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("/home/dah/.config/idiom/rust-analyzer");
    cmd
}

pub fn python_lsp() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("python3 -m pylsp");
    cmd
}
