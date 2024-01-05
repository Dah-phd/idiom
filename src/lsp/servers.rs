use tokio::process::Command;

const RUNNER: &str = "sh"; // TODO add configs for windows/macos

pub fn rust_lsp() -> Command {
    let mut cmd = Command::new(RUNNER);
    cmd.arg("-c").arg("/home/dah/.config/idiom/rust-analyzer");
    cmd
}

pub fn python_lsp() -> Command {
    let mut cmd = Command::new(RUNNER);
    cmd.arg("-c").arg("python3 -m pylsp");
    cmd
}
