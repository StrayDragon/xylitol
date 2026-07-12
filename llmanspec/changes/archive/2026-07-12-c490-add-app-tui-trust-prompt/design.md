# design — c490 trust ChoicePrompt

## 问题

今日：`bootstrap` 在 `TerminalGuard::enter` **之前**同步调用 `prompt_trust_options_stdio` → 用户看到的是 CLI，不是 TUI。

## 目标形态（对齐 agent_demo）

```
transcript（可空 / 系统说明）
────
ChoicePrompt  ← 替换 editor 槽（非大 overlay 仪表盘）
────
footer cwd · model
```

- 单题 Single：Trust / Trust parent / Do not trust（label 来自 `TrustManager::get_trust_options`）
- 主题：`Palette::dark().choice_prompt_theme()`
- Esc / cancel → 等同 deny（不信任）
- Enter 提交 → 写 store → 再加载项目资源

## Bootstrap 缝（推荐）

**推迟资源加载**，两段式：

1. **preflight + 构造 Driver 最小核**时：若需 Ask，**不**在 bootstrap 内 stdio 问；返回 `TrustPending { options, cwd }`（或 `BootstrapOutcome::NeedsTrust`）。
2. **TUI host 首帧**：挂 ChoicePrompt；用户决完后调用既有 `TrustManager` 写入，再 `reload_project_resources` / 完成原 bootstrap 后半段。

备选（更侵入）：bootstrap 注入 `on_prompt` 阻塞 channel，host 在 raw mode 里填答案——易死锁，**不推荐**。

print / server / `--no-trust` / 已存 store：**不变**（无 UI 或跳过）。

## 非目标

- 把 ChoicePrompt 做成居中大 overlay（DESIGN：短确认才 overlay；本选择器走 editor 槽）
- 改 trust 决议优先级（仍 `resolve_project_trusted`）
