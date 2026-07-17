# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-keybindings

  @req:pkb01
  场景: json-override-applies
    假如 Manager 已用默认 definitions 构造
    当 set_user_bindings 覆盖某 id 为新和弦
    那么 matches_event 对该 id 仅认新和弦

  @req:pkb01
  场景: unknown-id-ignored
    假如 用户配置含未知 id
    当 set_user_bindings
    那么 不 panic 且已知 id 行为不变

  @req:pkb02
  场景: default-without-override
    假如 无用户覆盖
    当 matches_event 使用 default_keys
    那么 与定义表一致
