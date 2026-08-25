# language: zh-CN
# capability: package-tui-keybindings
# purpose: xylitol-tui KeybindingsManager：静态 definitions、拥有型用户覆盖与按 id 匹配。
# scope: xylitol-tui 包, xylitol-tui 包测试

功能: package-tui-keybindings

  @req:pkb01 @human
  场景: owned-user-config
    - KeybindingsManager MUST 接受拥有型用户覆盖配置（键与和弦为 String），使磁盘 JSON 可热加载；definitions 的 id 仍为包/应用静态目录。set_user_bindings MUST 按 id 合并覆盖；未知 id MUST 忽略且 MUST NOT panic。

  @req:pkb02 @human
  场景: matches-by-id
    - KeybindingsManager::matches_event MUST 按绑定 id 解析当前生效和弦并与 KeyEvent 比较；默认键在无用户覆盖时 MUST 等于 definitions.default_keys。
