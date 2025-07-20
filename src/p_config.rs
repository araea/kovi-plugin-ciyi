use kovi::toml;
use kovi::utils::load_toml_data;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::p_config;

pub static CONFIG: OnceLock<p_config::Config> = OnceLock::new();

pub fn config() -> &'static p_config::Config {
    CONFIG.get().expect("Config not initialized")
}

pub const DEFAULT_CONFIG_STR: &str = r#"
# 群组过滤
[channel]

# 白名单群组，如果非空，则只在这些群组响应。
white = []
# 黑名单群组，在这些群组中插件将不响应。
black = ["123456789"]

# 插件配置
[plugin]

# 只有 @ Bot 时才回复
only_at = false 

# 指令前缀 示例：["!", "。"]
prefixes = [] 

# Bot 响应时 @ 用户
at_user = false

# Bot 响应时引用用户消息
quote_user = true

# 是否开启直接猜测模式（不需要指令，直接发送两字词语即可猜测）
direct_guess = false

# 提示中显示几个历史记录
history_display = 10

# 排行榜显示几个人
rank_display = 10
"#;

/// [channel]
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub white: Vec<String>,
    pub black: Vec<String>,
}

/// [plugin]
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    pub only_at: bool,
    pub prefixes: Vec<String>,
    pub at_user: bool,
    pub quote_user: bool,
    pub direct_guess: bool,
    pub history_display: usize,
    pub rank_display: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub channel: ChannelConfig,
    pub plugin: PluginConfig,

    #[serde(skip)]
    config_file_path: PathBuf,
}

impl Config {
    pub fn new(data_dir: PathBuf) -> Result<Self, Box<dyn Error>> {
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }

        let config_file_path = data_dir.join("config.toml");

        let default_config: Config = toml::from_str(DEFAULT_CONFIG_STR)?;

        let mut config: Config = load_toml_data(default_config, config_file_path.clone())?;

        config.config_file_path = config_file_path;

        Ok(config)
    }
}
