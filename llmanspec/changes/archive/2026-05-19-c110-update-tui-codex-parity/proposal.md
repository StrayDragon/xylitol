---
depends_on: [c80-add-tui, c90-refactor-tui-core, c91-add-tui-interactive, c92-add-tui-approval-diff, c105-fix-tui-initial-render]
---

# c110-update-tui-codex-parity

## Why

我们当前的 TUI（`src/interface/tui/`）在 MVP 上已经具备：事件驱动渲染、markdown、高亮、多行输入、工具卡片、approval/diff overlay 等能力；但整体交互仍与 `../codex`（codex-rs/tui）差距很大，导致“看起来能用但体验不对”：

- 缺少 **Codex 风格的 composer 交互语义**：
  - `Esc` 取消输入 / `Esc` `Esc` 回退并编辑上一条用户消息（backtrack）
  - `Tab` 在 task 运行时用于 queue；非运行时用于 autocomplete / 发送（并且 `!` shell 特例）
- 缺少 **slash popup / bang shell / file mention** 等核心机制：
  - `/` 打开命令列表并可 Tab 补全
  - `!` 进入 shell 输入模式（且 queue/submit 规则不同）
  - `@` 触发文件搜索/mention（为后续上下文注入与 tool 选择铺路）
- 缺少/不一致的 **全局快捷键集合与语义**（例如 Ctrl+O copy、Alt+R raw scrollback、Ctrl+T transcript overlay）
- 缺少 **可配置 keymap** 的抽象与冲突校验（Codex 有 runtime keymap resolution + reserved binding 体系）

本 change 的目标是“**以 xylitol 机制为主**（AgentEvent/ToolRegistry/Security/ReviewEngine 保持不变）”，在不触碰 `diff_review` comment/review UI 的前提下，补齐 codex-tui 交互语义与快捷键体系，使用户获得接近 codex 的肌肉记忆与操作流。

## What Changes

### 1) 引入 TUI Keymap（codex 风格但适配 xylitol）

- 新增 `src/interface/tui/keymap.rs`（或等价模块）定义：
  - `KeyBinding`（按键 + modifiers + 兼容匹配）
  - `RuntimeKeymap`（app/chat/composer/pager/list/approval 等 context）
  - 固定保留快捷键（如 Ctrl+C / Ctrl+D / '/' / '!' / '@'）
- App 层与组件不再直接 `match KeyCode`，改为通过 keymap dispatch。

### 2) 重写 composer / input 语义（对齐 codex）

- `Esc`：取消当前输入（清空 draft 或退出 popup），不退出程序。
- `Esc` `Esc`：进入 backtrack（选择上一条 user message → 预填 composer → 允许编辑重发）。
- `Tab`：
  - task 运行中：queue 当前输入（不打断当前 run）；
  - task 非运行中：
    - slash popup 激活时：补全选中命令；
    - 普通文本：保持现有 slash completion 或按 codex 语义扩展（以 keymap/状态机为准）；
    - `!` shell command：非运行中不应把 Tab 当 submit（保持 codex 特例）。

### 3) 增加 Codex 交互机制（但保留 xylitol backend）

- Slash popup：在输入以 `/` 开头或按 `/` 时弹出命令列表，支持 fuzzy/filter、Tab 补全、Enter 执行。
- Bang shell mode：输入以 `!` 开头时进入 shell 模式（表现与普通 prompt 不同，至少在 UI 上明确，并实现 queue/submit 规则）。
- File mention (`@`)：提供最小可用的文件搜索 overlay（先基于 `rg --files` / `glob` + 简单 fuzzy；后续可对接 LSP/skills）。

### 4) 增加 codex 常用全局快捷键（不涉及 review UI）

- `Ctrl+T`：Transcript overlay（全屏滚动 transcript，支持 q/Esc 退出）。
- `Ctrl+O`：Copy last assistant response as Markdown（macOS 可选对接 pbcopy；其它平台回落为提示 + 输出到临时文件）。
- `Alt+R`：Raw output mode（把 transcript 渲染为更适合终端选择/复制的格式；或切换 markdown 渲染策略）。

> 注：实现顺序按 tasks 分拆；每个子能力可独立验证并逐步集成。

## Capabilities

- `tui-interface`

## Impact

- 仅影响 `src/interface/tui/`（以及可能的 `Cargo.toml` 增加轻量依赖，如 clipboard/fuzzy 工具）
- `src/interface/diff_review/` 保持不动（用户明确要求不动 review diff comment 界面）
- 需要新增/更新 `tui-interface` spec delta，确保 keymap + composer 语义可验证
