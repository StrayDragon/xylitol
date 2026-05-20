---
depends_on: [c80-add-tui]
---

# c115-refactor-tui-codex-ui-parity

## Why

目前 xylitol 的 TUI 仍沿用“Chat/Tools/Input/StatusBar 盒子布局 + 边框”的壳与组件体系，即使补齐了一部分 codex 的快捷键与 composer 语义（Tab queue、Esc backtrack、Ctrl+T transcript、Ctrl+O copy、Alt+R raw 等），整体 **视觉结构与交互机制**仍与 `../codex/codex-rs/tui` 差距巨大，导致用户无法获得 codex 的肌肉记忆与操作流。

你明确要求：
- **全面复刻 ../codex 的 TUI 交互与逻辑（含快捷键与交互机制）**
- 但 **以 xylitol 的 agent/tool/security/review 机制为主**
- 并且 **不改 review diff comment 界面**（`src/interface/diff_review/` 保持不动）

要达成这一点，不能继续在现有 UI 壳上打补丁，必须在 `src/interface/tui/` 内进行一次结构级重写：把 UI 的“壳、布局、渲染管线、composer/footer/popup”迁移成 codex 风格，然后把这些 UI 意图（actions）再接回 xylitol 的 AgentLoop / ToolRegistry / ApprovalHub / ReviewEngine。

## What Changes

### 1) 新的 Codex-style UI Shell（替换现有三栏盒子布局）

- 引入一个新的根组件（例如 `ChatWidget` / `ChatSurface`），结构对齐 codex：
  - 顶部：可选 session header（或首条 session cell）
  - 中间：transcript viewport（包含 streaming、tool cards、thinking blocks、approval banners 等）
  - 底部：BottomPane（composer + footer + view stack）
- 移除（或保留但不再渲染）现有 `ToolPanelComponent` 与 `StatusBar` 的“独立面板”形式；改为 codex 风格的 footer/statusline。

### 2) 渲染风格对齐（无 Boxes，尽量遵循 codex styles.md）

- 默认不使用 `Borders::ALL` 的盒子分割。
- 使用 dim / cyan / magenta / green / red 等有限 ANSI 色板。
- Transcript 与 composer 使用 codex 风格的前缀（例如 `› `、`!`）与 wrap 规则。

### 3) 交互机制对齐（以 xylitol backend 为真值）

- 保留并强化已实现的 codex-style keymap + composer 行为：
  - Tab queue/submit（`!` 特例）
  - Esc cancel / Esc Esc backtrack
  - Ctrl+T transcript overlay
  - Ctrl+O copy last response
  - Alt+R raw output mode（真实 raw renderer，不只是 toggle flag）
- 将 overlays/pickers 迁移到 bottom-pane view stack 的交互模型，避免散落的 modal 逻辑。

### 4) Review UI 保持不动，仅做接线

- 任何 diff/review comment 的 UI 仍使用 `src/interface/diff_review/` 的既有实现。
- TUI 仅负责：在合适的时机弹出 review overlay、承接 approve/deny 结果、在 transcript 中记录摘要。

### 5) 许可证/归因（如复用 codex 源码）

如果直接移植 codex 的部分代码/文件（Apache-2.0）：
- 在仓库内新增第三方许可/归因文件（例如 `THIRD_PARTY_NOTICES.md` 或 `NOTICE`），包含来自 codex 的 NOTICE 与许可证文本。
- 对移植文件保留必要的版权声明，并在文件头增加“modified from …”的显著变更说明（满足 Apache-2.0 的 NOTICE/变更声明要求）。

## Capabilities

- `tui-interface`

## Impact

- 主要变更范围：`src/interface/tui/**`
- `src/interface/diff_review/**` 不修改
- 需要新增/更新 tui-interface 的 spec delta，覆盖：布局结构、footer/statusline、raw output 真正渲染、以及 codex-style 不带边框的 UI 风格约束。
