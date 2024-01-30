use crate::configs::get_config_dir;
use anyhow::{anyhow, Result};
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

pub fn server_cmd(mut lsp: String) -> Result<Command> {
    if lsp.contains("${cfg_dir}") {
        let cfg_dir = get_config_dir().ok_or(anyhow!("Failed to parse config dir!"))?.display().to_string();
        lsp = lsp.replace("${cfg_dir}", cfg_dir.as_str());
    }
    let mut cmd = Command::new(RUNNER);
    cmd.arg("-c").arg(lsp);
    Ok(cmd)
}
