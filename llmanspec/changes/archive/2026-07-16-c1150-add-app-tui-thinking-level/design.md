# Design — c1150-add-app-tui-thinking-level

## 已拍板

| 决策 | 选择 |
|---|---|
| Cycle 真源 | `ModelManager::cycle_thinking_level`（模型 `thinking_levels` 支持集顺序）；**禁止**产品用 `ThinkingBorderLevel::cycle_next` 七档硬转 |
| Driver 缝 | 新增 `Driver::cycle_thinking_level() -> Result<ThinkingLevel, String>`（内部调 manager）；读仍用 `thinking_level()`；可选继续支持 `set_thinking_level` / `Command::SetThinkingLevel` |
| 键位 | `app.thinking.cycle`，默认 `Shift+Tab`；经 `KeybindingsManager`（ati35）；与 `app.thinking.toggle`（Ctrl+T 折叠）正交 |
| Busy | **idle 与 busy 均可 cycle**（含 agent / bang busy）；无拒绝系统块；level 仅影响后续 turn |
| UX | **静默**：只更新边框 + footer；**MUST NOT** `push_system_note` / scrollback `thinking-border → …`；**MUST NOT** 产品 `/thinking-level` |
| Footer 文案 | `• thinking off`（level=off）或 `• {as_str}`（其余，如 `• xhigh`）；插在 `model` 与 token 字段之间 |
| 边框 | `ThinkingLevel` → `ThinkingBorderLevel`（按 `as_str` 1:1）；`apply_thinking_border`；bash 前缀覆盖；退出后恢复 thinking 而非 muted |
| 主题 | `set_layout_theme` / `reload_themes` 后按新 Palette 重涂 thinking 边框 |
| 模型切换 | SetModel / clamp 后从 `Driver::thinking_level()` 重同步 footer+边框 |
| 事件 | 本变更不依赖 `XyEvent::ThinkingLevelChanged` 发射；UI 在 cycle/set/model 切换路径同步读 Driver |

## 数据流

```text
Shift+Tab → KeybindingsManager(app.thinking.cycle)
  → host/input_policy 或 slot_input
  → Driver::cycle_thinking_level()
  → ModelManager::cycle_thinking_level
  → HostSession 同步：map level → ThinkingBorderLevel
       → apply_thinking_border(editor)
       → set_footer_thinking_label / format_footer_text
  → request_render
  （无 scrollback 写入）
```

## 与 demo 对照

| | agent_demo | 产品 |
|---|---|---|
| Cycle | 包 7 档 `cycle_next` | 模型支持集 |
| 反馈 | transcript + status | 边框 + footer only |
| Slash | `/thinking-level` | 禁止 |

## 风险

- ati15 历史 scenario「恢复 muted」需随 modify 更新为 thinking 边框
- FakeDriver / ScriptedDriver 须实现 `cycle_thinking_level`（可基于支持列表 stub）
