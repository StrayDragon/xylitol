---
change_id: c477-update-app-tui-demo-visual-parity
title: "app-tui-chrome：空闲操作区与 busy spinner 对齐 agent_demo"
status: ready
priority: 477
depends_on: ["c475-add-app-tui-chrome", "c476-add-app-tui-live-scrollback"]
author: agent
track: B
---

# c477-update-app-tui-demo-visual-parity

## Why

Trust / teardown / 基础 chrome 已通，但对照 `just demo-tui` 仍有可见落差：空闲时操作区固定 ~5 行空白显得「空盒子」；busy status 仍是纯文案、无 accent spinner；用户行可选背景尚未按 DESIGN 落地。需要一刀把 **视觉 parity** 收齐，避免和 c480（slash/steer）缠在一起。

## What Changes

1. **空闲操作区更紧**：空草稿时 Editor 可见行数贴近 agent_demo 观感（仍保留上下 `─`）；有草稿/多行时再长高，不破坏 `terminal_rows` 公式语义。
2. **Busy status**：独立一行 `accent spinner + 短词`（Working / Running tool…），idle 仍 0 行。
3. **User 行可选 `user-message-bg`**：按 `DESIGN.md` / `design/transcript.md` 给用户消息全行淡底（可开关或默认开、对齐 demo）。
4. **验收**：单测 + 对照 `just demo-tui` 的空闲/忙碌截图清单（写入 tasks）。

## Capabilities

- `app-tui-chrome`：增补 idle 操作区紧凑度、busy spinner、user-message-bg。

## Out of scope

- Slash / steer / abort / CompletionSource（**c480**）
- Bash `!` 边框（**c492**）
- 真 session 树（c491 stub 冻结）
- Trust ChoicePrompt 文案/退出策略（已落地）

## Impact

- 触达：`src/app/tui/{ui_root,scrollback,theme,tests}.rs`；或薄包 `Editor` 空态可见行 API（若必须改包，保持开闭、不改 ReAct）。
- 风险：压空闲高度时勿裁掉反色光标行；spinner 勿在 idle 漏帧。
