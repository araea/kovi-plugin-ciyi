mod ciyi_game;
mod p_command;
mod p_config;
mod p_const;
mod p_fn;

use kovi::chrono::Utc;
use kovi::tokio::sync::Mutex;
use std::sync::Arc;

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
                let command = COMMAND.get().unwrap();
                let config = CONFIG.get().unwrap();

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

                // 无意义消息过滤
                let text = match event.borrow_text() {
                    None => return,
                    Some(text) => text,
                };

                // 频道过滤
                if !p_fn::should_process_group(
                    &group_id,
                    &config.channel.white,
                    &config.channel.black,
                ) {
                    return;
                }

                // 直接猜测模式
                if text.chars().count() == 2 {
                    let mut game_manager = game_manager.lock().await;
                    let state = game_manager.get(&group_id).await;

                    if (state.last_start_time.date_naive() != Utc::now().date_naive()
                        || !state.is_finished)
                        && state.direct_guess_enabled
                    {
                        let response = p_fn::guess_word(&event, &[text], &mut game_manager).await;
                        p_fn::build_and_send_message(&event, &response);
                        return;
                    }
                }

                // 指令解析
                if let Some((cmd, params)) = p_fn::parse_command(text, &config.plugin.prefixes) {
                    if let Some(function) = command.get_function_by_command(cmd) {
                        let mut game_manager = game_manager.lock().await;
                        match function.as_str() {
                            "查看插件指令列表" => {
                                p_fn::build_and_send_message(&event, &p_fn::show_commands());
                            }
                            "查看词意游戏规则" => {
                                p_fn::build_and_send_message(&event, &p_fn::show_rules());
                            }
                            "猜测两字词语" => {
                                p_fn::build_and_send_message(
                                    &event,
                                    &p_fn::guess_word(&event, &params, &mut game_manager).await,
                                );
                            }
                            "查看当前频道的词意排行榜" => {
                                p_fn::build_and_send_message(
                                    &event,
                                    &game_manager.get_channel_leaderboard(&group_id),
                                );
                            }
                            "查看所有人的词意排行榜" => {
                                p_fn::build_and_send_message(
                                    &event,
                                    &game_manager.get_global_leaderboard(),
                                );
                            }
                            "切换猜测模式" => {
                                let response =
                                    game_manager.toggle_direct_guess_mode(&group_id).await;
                                p_fn::build_and_send_message(&event, &response);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    PluginBuilder::drop({
        let game_manager = Arc::clone(&game_manager);
        move || {
            let game_manager = Arc::clone(&game_manager);
            async move {
                game_manager.lock().await.save();
            }
        }
    });
}
