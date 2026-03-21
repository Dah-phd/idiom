use serde::{Deserialize, Serialize};
use toml::Table;

#[derive(Serialize, Deserialize, Debug)]
pub struct LSPConfig {
    cmd: String,
    preload_if_present: Option<Vec<String>>,
    init_cfg: Option<Table>,
}

impl LSPConfig {
    pub fn new(cmd: impl Into<String>, preload_if_present: Option<Vec<String>>, init_cfg: Option<Table>) -> Self {
        Self { cmd: cmd.into(), preload_if_present, init_cfg }
    }

    pub fn get_cmd(&self) -> String {
        self.cmd.to_owned()
    }

    pub fn take_preloads_markers(&mut self) -> Option<Vec<String>> {
        self.preload_if_present.take()
    }
}
