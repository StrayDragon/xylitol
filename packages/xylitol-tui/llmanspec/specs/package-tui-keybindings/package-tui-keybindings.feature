# language: zh-CN
# capability: package-tui-keybindings
# purpose: xylitol-tui KeybindingsManager：静态 definitions、拥有型用户覆盖与按 id 匹配。
# scope: packages/xylitol-tui/, packages/xylitol-tui/tests/

功能: package-tui-keybindings

  @req:r1641
  规则: owned-user-config
    KeybindingsManager MUST 接受拥有型用户覆盖配置（键与和弦为 String），使磁盘 JSON 可热加载；definitions 的 id 仍为包/应用静态目录。set_user_bindings MUST 按 id 合并覆盖；未知 id MUST 忽略且 MUST NOT panic。
    # verified-by: packages/xylitol-tui/tests/keybindings_test.rs
  @req:r1642
  规则: matches-by-id
    KeybindingsManager::matches_event MUST 按绑定 id 解析当前生效和弦并与 KeyEvent 比较；默认键在无用户覆盖时 MUST 等于 definitions.default_keys。

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: packages/xylitol-tui/tests/keybindings_test.rs
