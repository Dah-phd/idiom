use crate::configs::get_config_dir;
use anyhow::{anyhow, Result};
use tokio::process::Command;

const RUNNER: &str = "sh"; // TODO add configs for windows/macos

pub fn server_cmd(lsp: &str) -> Result<Command> {
    if lsp.contains("${cfg_dir}") {
        let cfg_dir = get_config_dir().ok_or(anyhow!("Failed to parse config dir!"))?.display().to_string();
        let mut cmd = Command::new(RUNNER);
        cmd.arg("-c").arg(lsp.replace("${cfg_dir}", cfg_dir.as_str()));
        return Ok(cmd)
    }
    let mut cmd = Command::new(RUNNER);
    cmd.arg("-c").arg(lsp);
    Ok(cmd)
}
