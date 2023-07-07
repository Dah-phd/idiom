use dirs::config_dir;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

const EDITOR_CONFIGS: &str = "/idiom/.editor";
const KEY_MAP: &str = "/idiom/.keys";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EditorConfigs {
    pub indent: String,
    pub indent_after: String,
}

impl Default for EditorConfigs {
    fn default() -> Self {
        if let Some(config_json) = read_config_file(EDITOR_CONFIGS) {
            match serde_json::from_slice::<EditorConfigs>(&config_json) {
                Ok(configs) => configs,
                Err(error) => {
                    match error.classify() {
                        Category::Data => {}
                        Category::Eof => {}
                        Category::Io => {}
                        Category::Syntax => {}
                    };
                    let configs = Self {
                        indent: "    ".to_owned(),
                        indent_after: ":({".to_owned(),
                    };
                    write_config_file(EDITOR_CONFIGS, &configs);
                    configs
                }
            }
        } else {
            let configs = Self {
                indent: "    ".to_owned(),
                indent_after: ":({".to_owned(),
            };
            write_config_file(EDITOR_CONFIGS, &configs).unwrap();
            configs
        }
    }
}

impl EditorConfigs {
    pub fn refresh(&mut self) {
        (*self) = Self::default()
    }
}

fn read_config_file(path: &str) -> Option<Vec<u8>> {
    let mut config_file = config_dir()?.into_os_string();
    config_file.push(path);
    std::fs::read(config_file).ok()
}

fn write_config_file<T: Serialize>(path: &str, configs: &T) -> Option<()> {
    let mut config_file = config_dir()?.into_os_string();
    config_file.push(path);
    let serialized = serde_json::to_string_pretty(configs).ok()?;
    std::fs::write(config_file, serialized).ok()
}
