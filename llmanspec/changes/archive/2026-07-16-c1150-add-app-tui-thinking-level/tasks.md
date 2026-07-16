# Tasks — c1150-add-app-tui-thinking-level

## 1. Driver 缝

- [x] 1.1 `Driver::cycle_thinking_level` → `ModelManager::cycle_thinking_level`；Fake/ScriptedDriver 实现
- [x] 1.2 单测：支持集内循环与 clamp 行为可观测

## 2. Chrome / layout

- [x] 2.1 `ThinkingLevel` → `ThinkingBorderLevel` 映射 + `UiRoot` 持有当前 level
- [x] 2.2 `sync_editor_border`：bash 覆盖；否则 `apply_thinking_border`；主题切换后重涂
- [x] 2.3 `format_footer_text`：插入 `• thinking off` / `• {level}`；host setter 同步

## 3. Input / host

- [x] 3.1 `keybindings.rs`：注册 `app.thinking.cycle` 默认 Shift+Tab
- [x] 3.2 idle/busy 输入路径触发 cycle → Driver → 静默更新 UI（无 system note）
- [x] 3.3 模型切换 / mount 时从 `Driver::thinking_level` 重同步

## 4. 文档与测

- [x] 4.1 更新 `design/footer.md`、`design/keybindings.md`
- [x] 4.2 harness：cycle 后 footer+边框一致；scrollback 无 thinking-border；busy 可 cycle；bash 退出恢复 thinking
- [x] 4.3 `llman sdd validate c1150-add-app-tui-thinking-level --strict --no-interactive`
- [x] 4.4 `just lint` + 相关 `harness` / lib 测
