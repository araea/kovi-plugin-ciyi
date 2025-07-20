use std::sync::Arc;

use kovi::{Message, MsgEvent};

use crate::{ciyi_game, p_command, p_config};

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
                format!("{} {}", prefix, first_cmd)
            }
        })
        .collect();

    command_lines.join("\n")
}

pub fn show_rules() -> String {
    format!(
        "\
目标
    猜出系统选择的两字词语

反馈
    每次猜测后，获得相似度排名与相邻词提示

    例如: `?好) 企业 (地? #467`
        #467      → 相似度排名 (越小越近)
        ?好 / 地? → 相邻词提示 (? 为隐藏字)

周期
    每日一词，猜对则次日刷新
    系统记录猜对次数，可查排行"
    )
}

pub async fn guess_word(
    event: &Arc<MsgEvent>,
    params: &[&str],
    game_manager: &mut ciyi_game::CiYiGameManager,
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

    game_manager
        .handle_guess(&group_id, &user_id, &username, guess_word)
        .await
}

pub fn should_process_group(group_id: &str, white_list: &[String], black_list: &[String]) -> bool {
    if black_list.contains(&group_id.to_string()) {
        return false;
    }

    if !white_list.is_empty() && !white_list.contains(&group_id.to_string()) {
        return false;
    }

    true
}

pub fn parse_command<'a>(text: &'a str, prefixes: &[String]) -> Option<(&'a str, Vec<&'a str>)> {
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
