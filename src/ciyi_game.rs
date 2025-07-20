use kovi::chrono::{DateTime, Utc};
use kovi::log;
use kovi::utils::{load_json_data, save_json_data};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
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
    pub async fn guess(&mut self, guess_word: String) -> (String, bool) {
        if self.is_finished {
            if Utc::now().date_naive() != self.last_start_time.date_naive() {
                let candidates: Vec<&str> = QUESTION_WORDS
                    .iter()
                    .map(|w| w.as_str())
                    .filter(|w| !self.global_history.contains(*w))
                    .collect();

                if candidates.is_empty() {
                    return ("题库已空".to_string(), false);
                }

                self.hints.clear();
                self.current_guesses.clear();
                self.target_word = candidates[fastrand::usize(..candidates.len())].to_string();
                match fetch_words_rank_list(&self.target_word).await {
                    Ok(list) => self.words_rank_list = list,
                    Err(e) => {
                        return (format!("获取词语排名失败：{}", e), false);
                    }
                }
                self.global_history.insert(self.target_word.clone());
                self.last_start_time = Utc::now();
                self.is_finished = false;
            } else {
                return ("今日已结束 请明天再来".to_string(), false);
            }
        }

        if self.words_rank_list.is_empty() {
            match fetch_words_rank_list(&self.target_word).await {
                Ok(list) => self.words_rank_list = list,
                Err(e) => {
                    return (format!("获取词语排名失败：{}", e), false);
                }
            }
        }

        if self.current_guesses.contains(&guess_word) {
            return (format!("{} 已猜过", guess_word), false);
        }

        if !ALL_WORDS.contains(&guess_word) {
            return (format!("{} 不在词库中", guess_word), false);
        }

        self.current_guesses.insert(guess_word.clone());

        if guess_word == self.target_word {
            self.is_finished = true;
            let success_message = format!(
                "恭喜你猜对了！\n答案：{}\n猜测：{} 次",
                self.target_word,
                self.current_guesses.len()
            );
            (success_message, true)
        } else {
            if let Some(index) = self.words_rank_list.iter().position(|w| w == &guess_word) {
                let rank = index + 1;
                let prev_char = self
                    .words_rank_list
                    .get(index.wrapping_sub(1))
                    .and_then(|w| w.chars().nth(1))
                    .map_or('？', |c| c);

                let next_char = self
                    .words_rank_list
                    .get(index + 1)
                    .and_then(|w| w.chars().next())
                    .map_or('？', |c| c);

                let hint_text = format!(
                    "？{} ) {} ( {}？ #{}",
                    prev_char, guess_word, next_char, rank
                );

                self.hints.push(Hint {
                    text: hint_text,
                    rank,
                });
            }

            self.hints.sort_unstable();

            let hints_str: String = self
                .hints
                .iter()
                .take(p_config::config().plugin.history_display)
                .enumerate()
                .map(|(i, hint)| format!("{}. {}\n", i + 1, hint.text))
                .collect();

            (format!("{}...", hints_str), false)
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CiYiGameManager {
    states: HashMap<String, CiYiGameState>,
    win_records: Vec<WinRecord>,

    #[serde(skip)]
    data_file_path: PathBuf,
}

unsafe impl Send for CiYiGameManager {}

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
        save_json_data(self, &self.data_file_path).unwrap();
    }

    pub async fn get(&mut self, channel_id: impl Into<String>) -> &mut CiYiGameState {
        let id = channel_id.into();

        if let Entry::Vacant(entry) = self.states.entry(id.clone()) {
            let target = &QUESTION_WORDS[fastrand::usize(..QUESTION_WORDS.len())];
            let new_state = CiYiGameState {
                channel_id: id.clone(),
                target_word: target.to_string(),
                last_start_time: Utc::now(),
                global_history: HashSet::from([target.to_string()]),
                current_guesses: HashSet::new(),
                words_rank_list: fetch_words_rank_list(target).await.unwrap_or_else(|_e| {
                    log::error!("Failed to fetch rank list for {}: {}", target, _e);
                    Vec::new()
                }),
                hints: Vec::new(),
                is_finished: false,
                direct_guess_enabled: p_config::config().plugin.direct_guess,
            };
            entry.insert(new_state);
        }

        self.states.get_mut(&id).unwrap()
    }

    pub async fn toggle_direct_guess_mode(&mut self, channel_id: &str) -> String {
        let state = self.get(channel_id).await;
        state.direct_guess_enabled = !state.direct_guess_enabled;

        if state.direct_guess_enabled {
            "直接猜测模式 已开启".to_string()
        } else {
            "直接猜测模式 已关闭".to_string()
        }
    }

    pub async fn handle_guess(
        &mut self,
        channel_id: &str,
        user_id: &str,
        username: &str,
        guess_word: String,
    ) -> String {
        let state = self.get(channel_id).await;
        let (message, is_win) = state.guess(guess_word).await;

        if is_win {
            self.win_records.push(WinRecord {
                user_id: user_id.to_string(),
                username: username.to_string(),
                channel_id: channel_id.to_string(),
                timestamp: Utc::now(),
            });
        }
        message
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
            let user_score = scores
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

async fn fetch_words_rank_list(word: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let url = format!(
        "https://ci-ying.oss-cn-zhangjiakou.aliyuncs.com/v1/ci-yi-list/{}.txt",
        word
    );
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
