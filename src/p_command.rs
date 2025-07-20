use kovi::toml;
use kovi::utils::load_toml_data;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::p_command;

pub static COMMAND: OnceLock<p_command::CommandConfig> = OnceLock::new();
pub fn commands() -> &'static p_command::CommandConfig {
    COMMAND.get().expect("Commands not initialized")
}

pub const DEFAULT_COMMANDS_STR: &str = r#"
# 定义插件的指令。每个 [[command]] 块代表一种功能及其关联的触发词。
# function: 功能的内部描述，用于代码逻辑判断。
# commands: 用户可以输入的指令列表。

[[command]]
function = "查看插件指令列表"
commands = ["词意指令", "词意帮助", "词意指令列表", "词意帮助列表"]

[[command]]
function = "查看词意游戏规则"
commands = ["词意规则", "词意玩法"]

[[command]]
function = "猜测两字词语"
commands = ["词意猜测"]

[[command]]
function = "查看当前频道的词意排行榜"
commands = ["词意榜"]

[[command]]
function = "查看所有人的词意排行榜"
commands = ["词意全榜"]

[[command]]
function = "切换猜测模式"
commands = ["切换猜测模式"]
"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandEntry {
    pub function: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandConfig {
    pub command: Vec<CommandEntry>,

    #[serde(skip)]
    config_file_path: PathBuf,
}

impl CommandConfig {
    pub fn new(data_dir: PathBuf) -> Result<Self, Box<dyn Error>> {
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }

        let config_file_path = data_dir.join("command.toml");

        let default_config: CommandConfig = toml::from_str(DEFAULT_COMMANDS_STR)?;

        let mut config: CommandConfig = load_toml_data(default_config, config_file_path.clone())?;

        config.config_file_path = config_file_path;

        Ok(config)
    }

    pub fn get_function_by_command(&self, cmd_str: &str) -> Option<&String> {
        for entry in &self.command {
            if entry.commands.iter().any(|cmd| cmd == cmd_str) {
                return Some(&entry.function);
            }
        }
        None
    }
}
