use crate::{
    configs::get_config_dir,
    lsp::{LSPError, LSPResult},
    utils::SHELL,
};
use tokio::process::Command;

pub fn server_cmd(lsp: &str) -> LSPResult<Command> {
    if lsp.contains("${cfg_dir}") {
        let cfg_dir = get_config_dir().ok_or(LSPError::internal("Failed to find config dir!"))?.display().to_string();
        let mut cmd = Command::new(SHELL);
        cmd.arg("-c").arg(lsp.replace("${cfg_dir}", cfg_dir.as_str()));
        return Ok(cmd);
    }
    let mut cmd = Command::new(SHELL);
    cmd.arg("-c").arg(lsp);
    Ok(cmd)
}
