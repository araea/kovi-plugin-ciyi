kovi-plugin-ciyi
================

[<img alt="github" src="https://img.shields.io/badge/github-araea/kovi_plugin_ciyi-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/araea/kovi-plugin-ciyi)
[<img alt="crates.io" src="https://img.shields.io/crates/v/kovi-plugin-ciyi.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/kovi-plugin-ciyi)

Kovi 词意插件 · 猜词游戏
通过猜测两字词语，根据相似度提示找出目标词语

## 游戏规则

```
目标
  猜出系统选择的两字词语

反馈
  每次猜测后获得：
  - 与目标词语的相似度排名
  - 相邻词提示

示例
  1. ？器 ) 镯子 ( 玉？   #14
  2. ？子 ) 玉佩 ( 东？   #15
  3. ？佩 ) 东西 ( 冥？   #16

  #14   → 相似度排名（越小越近）
  玉？   → 相邻词提示（？为"佩"）

周期
  每日一词，猜对则次日刷新
  系统记录猜对次数，可查排行
```

## 前置

1. 创建 Kovi 项目
2. 执行 `cargo kovi add ciyi`
3. 在 `src/main.rs` 中添加 `kovi_plugin_ciyi`

## 使用

1. 发送 `词意指令` 查看所有指令
2. 发送 `词意猜测 词语`，如 `词意猜测 企业`，获取提示
3. 根据提示继续猜测，直到找出正确答案
4. 可开启直接猜测模式，无需输入指令前缀
5. 结合 `词意帮助` 与 `词意规则` 自行探索

## 配置

资源目录：`data/kovi-plugin-ciyi/*`
> 首次运行时自动生成

### `config.toml` - 插件配置

```toml
# 群组过滤
[channel]

# 白名单群组，如果非空，则只在这些群组响应
white = []
# 黑名单群组，在这些群组中插件将不响应
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
```

### `command.toml` - 指令配置

```toml
[[command]]
# 功能（勿改）
function = "插件指令列表"
# 指令名（可增删）
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
```

## 致谢

- [Kovi](https://kovi.threkork.com/)
- [词影](https://cy.surprising.studio/)

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
