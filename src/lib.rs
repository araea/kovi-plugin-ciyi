//! CiYi Game - An all-in-one Rust plugin file.

// =============================
//          Modules
// =============================

mod ciyi_game {
    use kovi::chrono::{DateTime, Duration, Utc};
    use kovi::log;
    use kovi::utils::{load_json_data, save_json_data};
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;

    use std::collections::{HashMap, HashSet};
    use std::error::Error;
    use std::path::PathBuf;

    use crate::p_config;
    use crate::p_const::ALL_WORDS;
    use crate::p_const::QUESTION_WORDS;

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct UserScore {
        pub user_id: String,
        pub username: String,
        pub score: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WinRecord {
        pub user_id: String,
        pub username: String,
        pub channel_id: String,
        #[serde(with = "chrono::serde::ts_seconds")]
        pub timestamp: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
    pub struct Hint {
        pub text: String,
        pub rank: usize,
    }

    impl Ord for Hint {
        fn cmp(&self, other: &Self) -> Ordering {
            self.rank.cmp(&other.rank)
        }
    }

    impl PartialOrd for Hint {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CiYiGameState {
        pub channel_id: String,
        pub target_word: String,
        #[serde(with = "chrono::serde::ts_seconds")]
        pub last_start_time: DateTime<Utc>,
        pub global_history: HashSet<String>,
        pub current_guesses: HashSet<String>,
        pub words_rank_list: Vec<String>,
        pub hints: Vec<Hint>,
        pub is_finished: bool,
        #[serde(default)]
        pub direct_guess_enabled: bool,
    }

    impl CiYiGameState {
        pub fn is_new_day_in_china_timezone(&self) -> bool {
            const CHINA_TIMEZONE_OFFSET_HOURS: i64 = 8;
            let now_in_china_tz = Utc::now() + Duration::hours(CHINA_TIMEZONE_OFFSET_HOURS);
            let last_start_in_china_tz =
                self.last_start_time + Duration::hours(CHINA_TIMEZONE_OFFSET_HOURS);
            now_in_china_tz.date_naive() != last_start_in_china_tz.date_naive()
        }
    }

    #[derive(Debug)]
    pub enum FetchReason {
        NewGame,
        NewDay,
        MissingRankList,
    }

    #[derive(Debug)]
    pub struct FetchRequest {
        pub word_to_fetch: String,
        pub reason: FetchReason,
    }

    pub struct FetchedData {
        pub request: FetchRequest,
        pub result: Result<Vec<String>, Box<dyn Error>>,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct CiYiGameManager {
        states: HashMap<String, CiYiGameState>,
        win_records: Vec<WinRecord>,
        #[serde(skip)]
        data_file_path: PathBuf,
    }

    impl CiYiGameManager {
        pub fn new(data_dir: PathBuf) -> Result<Self, Box<dyn Error>> {
            if !data_dir.exists() {
                std::fs::create_dir_all(&data_dir)?;
            }
            let data_file_path = data_dir.join("ciyi_game_data.json");
            let mut manager: CiYiGameManager =
                load_json_data(CiYiGameManager::default(), data_file_path.clone())?;
            manager.data_file_path = data_file_path;
            Ok(manager)
        }

        pub fn save(&self) {
            if let Err(e) = save_json_data(self, &self.data_file_path) {
                log::error!("Failed to save ciyi game data: {e}");
            }
        }

        pub fn prepare_guess(&self, channel_id: &str) -> Option<FetchRequest> {
            let state = match self.states.get(channel_id) {
                Some(s) => s,
                None => {
                    let target = &QUESTION_WORDS[fastrand::usize(..QUESTION_WORDS.len())];
                    return Some(FetchRequest {
                        word_to_fetch: target.to_string(),
                        reason: FetchReason::NewGame,
                    });
                }
            };

            if state.is_finished && state.is_new_day_in_china_timezone() {
                let candidates: Vec<&str> = QUESTION_WORDS
                    .iter()
                    .filter(|w| !state.global_history.contains(w.as_str()))
                    .map(|w| w.as_str())
                    .collect();

                if candidates.is_empty() {
                    return None;
                }

                let new_target = candidates[fastrand::usize(..candidates.len())].to_string();
                return Some(FetchRequest {
                    word_to_fetch: new_target,
                    reason: FetchReason::NewDay,
                });
            }

            if !state.is_finished && state.words_rank_list.is_empty() {
                return Some(FetchRequest {
                    word_to_fetch: state.target_word.clone(),
                    reason: FetchReason::MissingRankList,
                });
            }

            None
        }

        pub fn commit_guess(
            &mut self,
            channel_id: &str,
            user_id: &str,
            username: &str,
            guess_word: String,
            fetched_data: Option<FetchedData>,
        ) -> String {
            if let Some(data) = fetched_data {
                let rank_list = match data.result {
                    Ok(list) => list,
                    Err(e) => return format!("获取词语排名失败：{e}"),
                };

                match data.request.reason {
                    FetchReason::NewGame => {
                        let new_state = CiYiGameState {
                            channel_id: channel_id.to_string(),
                            target_word: data.request.word_to_fetch.clone(),
                            last_start_time: Utc::now(),
                            global_history: HashSet::from([data.request.word_to_fetch.clone()]),
                            current_guesses: HashSet::new(),
                            words_rank_list: rank_list,
                            hints: Vec::new(),
                            is_finished: false,
                            direct_guess_enabled: p_config::config().plugin.direct_guess,
                        };
                        self.states.insert(channel_id.to_string(), new_state);
                    }
                    FetchReason::NewDay => {
                        if let Some(state) = self.states.get_mut(channel_id) {
                            state.hints.clear();
                            state.current_guesses.clear();
                            state.target_word = data.request.word_to_fetch.clone();
                            state.global_history.insert(data.request.word_to_fetch);
                            state.words_rank_list = rank_list;
                            state.last_start_time = Utc::now();
                            state.is_finished = false;
                        }
                    }
                    FetchReason::MissingRankList => {
                        if let Some(state) = self.states.get_mut(channel_id) {
                            state.words_rank_list = rank_list;
                        }
                    }
                }
            }

            let state = match self.states.get_mut(channel_id) {
                Some(s) => s,
                None => return "游戏尚未开始，请重试".to_string(),
            };

            if state.is_finished {
                return "每天只能玩一次哦！".to_string();
            }

            if state.current_guesses.contains(&guess_word) {
                return format!("{guess_word} 已猜过");
            }

            if !ALL_WORDS.contains(&guess_word) {
                return format!("{guess_word} 不在词库中");
            }

            state.current_guesses.insert(guess_word.clone());

            if guess_word == state.target_word {
                state.is_finished = true;
                self.win_records.push(WinRecord {
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    channel_id: channel_id.to_string(),
                    timestamp: Utc::now(),
                });
                format!(
                    "恭喜你猜对了！\n答案：{}\n猜测：{} 次",
                    state.target_word,
                    state.current_guesses.len()
                )
            } else {
                if let Some(index) = state.words_rank_list.iter().position(|w| w == &guess_word) {
                    let rank = index + 1;
                    let prev_char = state
                        .words_rank_list
                        .get(index.wrapping_sub(1))
                        .and_then(|w| w.chars().nth(1))
                        .map_or('？', |c| c);
                    let next_char = state
                        .words_rank_list
                        .get(index + 1)
                        .and_then(|w| w.chars().next())
                        .map_or('？', |c| c);
                    let hint_text = format!("？{prev_char} ) {guess_word} ( {next_char}？ #{rank}");
                    state.hints.push(Hint {
                        text: hint_text,
                        rank,
                    });
                }
                state.hints.sort_unstable();
                let hints_str: String = state
                    .hints
                    .iter()
                    .take(p_config::config().plugin.history_display)
                    .enumerate()
                    .map(|(i, hint)| format!("{}. {}\n", i + 1, hint.text))
                    .collect();
                format!("{hints_str}...")
            }
        }

        pub fn get_direct_guess_status(&mut self, channel_id: &str) -> bool {
            let state = self.states.get(channel_id);
            match state {
                Some(s) => {
                    (s.is_new_day_in_china_timezone() || !s.is_finished) && s.direct_guess_enabled
                }
                None => p_config::config().plugin.direct_guess,
            }
        }

        pub fn toggle_direct_guess_mode(&mut self, channel_id: &str) -> String {
            let state = self
                .states
                .entry(channel_id.to_string())
                .or_insert_with(|| {
                    let target = &QUESTION_WORDS[fastrand::usize(..QUESTION_WORDS.len())];
                    CiYiGameState {
                        channel_id: channel_id.to_string(),
                        target_word: target.to_string(),
                        last_start_time: Utc::now(),
                        global_history: HashSet::from([target.to_string()]),
                        current_guesses: HashSet::new(),
                        words_rank_list: Vec::new(),
                        hints: Vec::new(),
                        is_finished: false,
                        direct_guess_enabled: p_config::config().plugin.direct_guess,
                    }
                });

            state.direct_guess_enabled = !state.direct_guess_enabled;

            if state.direct_guess_enabled {
                "直接猜测模式 已开启".to_string()
            } else {
                "直接猜测模式 已关闭".to_string()
            }
        }

        pub fn get_global_leaderboard(&self) -> String {
            self.generate_leaderboard(self.win_records.iter())
        }

        pub fn get_channel_leaderboard(&self, channel_id: &str) -> String {
            let channel_records = self
                .win_records
                .iter()
                .filter(|r| r.channel_id == channel_id);
            self.generate_leaderboard(channel_records)
        }

        fn generate_leaderboard<'a, I>(&self, records: I) -> String
        where
            I: Iterator<Item = &'a WinRecord>,
        {
            let mut scores: HashMap<String, UserScore> = HashMap::new();
            for record in records {
                let user_score =
                    scores
                        .entry(record.user_id.clone())
                        .or_insert_with(|| UserScore {
                            user_id: record.user_id.clone(),
                            username: record.username.clone(),
                            score: 0,
                        });
                user_score.username = record.username.clone();
                user_score.score += 1;
            }

            if scores.is_empty() {
                return "当前还没有人猜对过哦！".to_string();
            }

            let mut sorted_scores: Vec<UserScore> = scores.into_values().collect();
            sorted_scores.sort_by(|a, b| b.score.cmp(&a.score));

            let leaderboard_str: String = sorted_scores
                .iter()
                .take(p_config::config().plugin.rank_display)
                .enumerate()
                .map(|(index, user_score)| {
                    format!(
                        "{}. {} {}",
                        index + 1,
                        user_score.username,
                        user_score.score
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");

            leaderboard_str
        }
    }

    pub async fn fetch_words_rank_list(word: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let url =
            format!("https://ci-ying.oss-cn-zhangjiakou.aliyuncs.com/v1/ci-yi-list/{word}.txt");
        let response = reqwest::get(&url).await?;
        let response = response.error_for_status()?;
        let body_text = response.text().await?;
        let words_rank_list: Vec<String> = body_text
            .trim()
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Ok(words_rank_list)
    }
}

mod p_command {
    use kovi::toml;
    use kovi::utils::load_toml_data;
    use serde::{Deserialize, Serialize};
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    pub static COMMAND: OnceLock<CommandConfig> = OnceLock::new();
    pub fn commands() -> &'static CommandConfig {
        COMMAND.get().expect("Commands not initialized")
    }

    pub const DEFAULT_COMMANDS_STR: &str = r#"
# 定义插件的指令。每个 [[command]] 块代表一种功能及其关联的触发词。
# function: 功能的内部描述，用于代码逻辑判断。
# commands: 用户可以输入的指令列表。

[[command]]
function = "查看插件指令列表"
commands = ["词意帮助", "词意指令", "词意指令列表", "词意帮助列表"]

[[command]]
function = "查看词意游戏规则"
commands = ["词意玩法", "词意规则"]

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

            let mut config: CommandConfig =
                load_toml_data(default_config, config_file_path.clone())?;

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
}

mod p_config {
    use kovi::toml;
    use kovi::utils::load_toml_data;
    use serde::{Deserialize, Serialize};
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    pub static CONFIG: OnceLock<Config> = OnceLock::new();

    pub fn config() -> &'static Config {
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
}

mod p_const {
    use kovi::serde_json;
    use once_cell::sync::Lazy;
    use std::collections::HashSet;

    const ALL_WORDS_JSON: &str = include_str!("../res/all_words.json");
    const QUESTION_WORDS_JSON: &str = include_str!("../res/question_words.json");

    pub static ALL_WORDS: Lazy<HashSet<String>> = Lazy::new(|| {
        let words: Vec<String> =
            serde_json::from_str(ALL_WORDS_JSON).expect("Failed to parse all_words.json");
        words.into_iter().collect()
    });

    pub static QUESTION_WORDS: Lazy<Vec<String>> = Lazy::new(|| {
        serde_json::from_str(QUESTION_WORDS_JSON).expect("Failed to parse question_words.json")
    });
}

mod p_fn {
    use std::sync::{Arc, Mutex};

    use kovi::{Message, MsgEvent};

    use crate::{
        ciyi_game::{self, CiYiGameManager, FetchedData},
        p_command, p_config,
    };

    pub fn show_commands() -> String {
        let config = p_config::config();
        let command = p_command::commands();

        let prefix: &'static str = config.plugin.prefixes.first().map_or("", |p| p.as_str());

        let command_lines: Vec<String> = command
            .command
            .iter()
            .map(|entry| {
                let first_cmd = entry
                    .commands
                    .first()
                    .map_or(format!("{}(禁用)", entry.function), |cmd| cmd.clone());

                if prefix.is_empty() {
                    first_cmd.to_string()
                } else {
                    format!("{prefix} {first_cmd}")
                }
            })
            .collect();

        command_lines.join("\n")
    }

    pub fn show_rules() -> String {
        "\
目标
    猜出系统选择的两字词语

反馈
    每次猜测后，获得：
    - 与目标词语的相似度排名
    - 相邻词提示

示例
    1. ？器 ) 镯子 ( 玉？   #14
    2. ？子 ) 玉佩 ( 东？   #15
    3. ？佩 ) 东西 ( 冥？   #16

    #14   → 相似度排名（越小越近）
    玉？   → 相邻词提示（？为“佩”）

周期
    每日一词，猜对则次日刷新
    系统记录猜对次数，可查排行"
            .to_string()
    }

    pub async fn guess_word(
        event: &Arc<MsgEvent>,
        params: &[&str],
        game_manager_mutex: &Arc<Mutex<CiYiGameManager>>,
    ) -> String {
        if params.is_empty() || params[0].chars().count() != 2 {
            return format!("无效输入：{}", params[0]);
        }

        let guess_word = params[0].to_string();
        let group_id = event.group_id.unwrap().to_string();
        let user_id = event.user_id.to_string();
        let username = event
            .sender
            .nickname
            .clone()
            .unwrap_or_else(|| event.sender.user_id.to_string());

        let fetch_request = {
            let manager = game_manager_mutex.lock().unwrap();
            manager.prepare_guess(&group_id)
        };

        let fetched_data = if let Some(req) = fetch_request {
            let result = ciyi_game::fetch_words_rank_list(&req.word_to_fetch).await;
            Some(FetchedData {
                request: req,
                result,
            })
        } else {
            None
        };

        {
            let mut manager = game_manager_mutex.lock().unwrap();
            manager.commit_guess(&group_id, &user_id, &username, guess_word, fetched_data)
        }
    }

    pub fn should_process_group(
        group_id: &str,
        white_list: &[String],
        black_list: &[String],
    ) -> bool {
        if black_list.contains(&group_id.to_string()) {
            return false;
        }

        if !white_list.is_empty() && !white_list.contains(&group_id.to_string()) {
            return false;
        }

        true
    }

    pub fn parse_command<'a>(
        text: &'a str,
        prefixes: &[String],
    ) -> Option<(&'a str, Vec<&'a str>)> {
        let mut words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return None;
        }

        let mut command = words.remove(0);

        if prefixes.is_empty() {
            Some((command, words))
        } else {
            let mut sorted_prefixes = prefixes.to_vec();
            sorted_prefixes.sort_by_key(|b| std::cmp::Reverse(b.len()));

            command = sorted_prefixes
                .iter()
                .find(|&p| command.starts_with(p))
                .map(|p| &command[p.len()..])?;

            Some((command, words))
        }
    }

    pub fn build_and_send_message(event: &Arc<MsgEvent>, msg: &str) {
        let config = p_config::config();
        let message = match (config.plugin.at_user, config.plugin.quote_user) {
            (true, false) => Message::new()
                .add_at(&event.user_id.to_string())
                .add_text("\n")
                .add_text(msg),
            (false, true) => Message::new().add_reply(event.message_id).add_text(msg),
            (true, true) => Message::new()
                .add_reply(event.message_id)
                .add_at(&event.user_id.to_string())
                .add_text("\n")
                .add_text(msg),
            (false, false) => Message::new().add_text(msg),
        };

        event.reply(message);
    }
}

// =============================
//      Main Plugin Logic
// =============================

use std::sync::{Arc, Mutex};

use kovi::PluginBuilder;

use crate::{p_command::COMMAND, p_config::CONFIG};

#[kovi::plugin]
async fn main() {
    let bot = PluginBuilder::get_runtime_bot();
    let data_dir = bot.get_data_path();
    let game_manager = Arc::new(Mutex::new(
        ciyi_game::CiYiGameManager::new(data_dir.clone()).unwrap(),
    ));

    COMMAND
        .set(p_command::CommandConfig::new(data_dir.clone()).unwrap())
        .unwrap();
    CONFIG
        .set(p_config::Config::new(data_dir.clone()).unwrap())
        .unwrap();

    PluginBuilder::on_msg({
        let game_manager = Arc::clone(&game_manager);

        move |event| {
            let game_manager = Arc::clone(&game_manager);

            async move {
                let command_map = p_command::commands();
                let config = p_config::config();

                let group_id = match event.group_id {
                    Some(id) => id.to_string(),
                    None => return, // 仅处理群组消息
                };

                // 仅 @机器人 时响应
                if config.plugin.only_at {
                    let message = &event.message;
                    let segment = message.get_from_index(0).unwrap();
                    if segment.type_ != "at"
                        || segment.data["qq"].as_str().unwrap().parse::<i64>().unwrap()
                            != event.self_id
                    {
                        return;
                    }
                }

                let text = match event.borrow_text() {
                    Some(text) if !text.trim().is_empty() => text,
                    _ => return, // 过滤空消息或无文本消息
                };

                if !p_fn::should_process_group(
                    &group_id,
                    &config.channel.white,
                    &config.channel.black,
                ) {
                    return;
                }

                // 直接猜测模式
                if text.chars().count() == 2 {
                    let should_direct_guess = {
                        let mut manager = game_manager.lock().unwrap();
                        manager.get_direct_guess_status(&group_id)
                    };
                    if should_direct_guess {
                        let response = p_fn::guess_word(&event, &[text], &game_manager).await;
                        p_fn::build_and_send_message(&event, &response);
                        return;
                    }
                }

                // 指令解析
                if let Some((cmd, params)) = p_fn::parse_command(text, &config.plugin.prefixes)
                    && let Some(function) = command_map.get_function_by_command(cmd) {
                        match function.as_str() {
                            "查看插件指令列表" => {
                                p_fn::build_and_send_message(&event, &p_fn::show_commands());
                            }
                            "查看词意游戏规则" => {
                                p_fn::build_and_send_message(&event, &p_fn::show_rules());
                            }
                            "猜测两字词语" => {
                                let response =
                                    p_fn::guess_word(&event, &params, &game_manager).await;
                                p_fn::build_and_send_message(&event, &response);
                            }
                            "查看当前频道的词意排行榜" => {
                                let leaderboard = {
                                    let manager = game_manager.lock().unwrap();
                                    manager.get_channel_leaderboard(&group_id)
                                };
                                p_fn::build_and_send_message(&event, &leaderboard);
                            }
                            "查看所有人的词意排行榜" => {
                                let leaderboard = {
                                    let manager = game_manager.lock().unwrap();
                                    manager.get_global_leaderboard()
                                };
                                p_fn::build_and_send_message(&event, &leaderboard);
                            }
                            "切换猜测模式" => {
                                let response = {
                                    let mut manager = game_manager.lock().unwrap();
                                    manager.toggle_direct_guess_mode(&group_id)
                                };
                                p_fn::build_and_send_message(&event, &response);
                            }
                            _ => {}
                        }
                    }
            }
        }
    });

    PluginBuilder::drop({
        let game_manager = Arc::clone(&game_manager);
        move || {
            let game_manager_clone = Arc::clone(&game_manager);
            async move {
                game_manager_clone.lock().unwrap().save();
            }
        }
    });
}
