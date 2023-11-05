use tokio::process::Command;

const TEST: usize = 3;

pub fn start_lsp() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("/home/dah/.config/idiom/rust-analyzer");
    cmd
}
