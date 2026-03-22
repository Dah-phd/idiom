use crate::lsp::servers::InitCfg;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Jval};
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

    pub fn new_no_semantic_tokens(cmd: impl Into<String>, preload_if_present: Option<Vec<String>>) -> Self {
        let mut text_document = Table::new();
        text_document.insert("semanticTokens".into(), toml::Value::String("null".into()));
        let mut capabilities = Table::new();
        capabilities.insert("textDocument".into(), toml::Value::Table(text_document));
        let mut init = Table::new();
        init.insert("capabilities".into(), toml::Value::Table(capabilities));
        Self { cmd: cmd.into(), preload_if_present, init_cfg: Some(init) }
    }

    pub fn get_cmd(&self) -> String {
        self.cmd.to_owned()
    }

    pub fn get_cmd_with_configs(&self) -> (String, InitCfg) {
        (self.get_cmd(), self.get_init_config())
    }

    pub fn get_init_config(&self) -> Option<Map<String, Jval>> {
        self.init_cfg.clone().map(|t| t.into_iter().map(|(k, v)| (k, convert_value(v))).collect())
    }

    pub fn take_preloads_markers(&mut self) -> Option<Vec<String>> {
        self.preload_if_present.take()
    }
}

fn convert_value(toml_values: toml::Value) -> serde_json::Value {
    match toml_values {
        toml::Value::String(text) if text == "null" => serde_json::Value::Null,
        toml::Value::String(text) => serde_json::Value::String(text),
        toml::Value::Integer(num) => serde_json::Value::Number(num.into()),
        toml::Value::Float(float) => serde_json::Value::Number(
            // ensure number coersion - extremely unlikely to happen
            serde_json::Number::from_f64(float).unwrap_or((float as i64).into()),
        ),
        toml::Value::Boolean(bool) => serde_json::Value::Bool(bool),
        toml::Value::Datetime(datetime) => serde_json::Value::String(datetime.to_string()),
        toml::Value::Array(list) => serde_json::Value::Array(list.into_iter().map(convert_value).collect()),
        toml::Value::Table(table) => {
            serde_json::Value::Object(table.into_iter().map(|(k, v)| (k, convert_value(v))).collect())
        }
    }
}
