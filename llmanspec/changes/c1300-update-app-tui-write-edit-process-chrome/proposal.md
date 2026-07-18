---
change_id: c1300-update-app-tui-write-edit-process-chrome
title: app-tui：write/edit 过程确认 chrome（流式正文、合块、去 [ok]）
status: full
priority: 1300
depends_on:
- c1280-update-app-tui-tool-chrome-quiet-defaults
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1300-update-app-tui-write-edit-process-chrome

## Why

1. 对照 pi：write 在意图流式阶段即显示正文（默认 10 行 viewport + ctrl+o）；edit 为**单块** header+diff，就绪后直接可见，无需 Alt+E。
2. xylitol 图证：write 仅头行 `(N lines)`、成功 output 已 quiet 为空；edit 拆成 `edit [ok]…` + `edited…` 两块，默认折叠。
3. 头上 `[ok]`/`[err]` 噪声；pi 用 tint + 失败末行表达成败。
4. 复用已有 `ExpandableOutput` / Ctrl+O；产品侧需解除「先 Alt+E 才见过程」闸对 write/edit 过程确认的阻碍。

## Purpose

固定产品过程确认行为：

1. **write**：MessageUpdate/ToolCall 意图阶段把 `content` 写入可渲染 body；默认 **Head 10 行** viewport + `… (N more, total, ctrl+o)`；路径 `~` 缩短；头行 `write <path>`（无重复 `write write`、无 `[ok]`）。
2. **edit**：成功后 **单块**（header + diff body）；**MUST NOT** 再推独立 `UiEntry::Diff` 第二块；diff 就绪后默认可见（对齐 pi：edit 不走 Alt+E 折叠）。
3. **成败**：去掉头上 `[ok]`/`[err]`；pending/success/error 靠 tint；失败文案在块末行。
4. Alt+E 可保留给其它工具详情；write viewport 用 Ctrl+O；edit 完整 diff 默认展开。

## What Changes

- `src/app/tui/bridge`：Tool 模型承载 write body / edit display_diff；停止成功 edit 的第二 Diff push
- `src/app/tui/widgets/scrollback.rs`：write Head-10；edit 合块默认显 body；去 `[ok]`；路径缩短
- live specs：`app-tui-bridge` / `app-tui-transcript`（修改 atb11 合块语义；新增过程确认 req）
- 单测 + 可选 agent_demo 对照

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-transcript`（modify）

## Out of scope

- infra tool_result 瘦身 → **c1310**
- **路径流式 / read 行域 / 粘性 path** → **c1320**
- XML tool salvage、fastrace、假树 stub
- 全工具语法高亮（write 可先纯文本；高亮可选增量）

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒完整 JSON；假树扩活树；reach agent/infra 内部
- required_evidence: bridge/scrollback 单测 — write 意图帧含 viewport；edit 成功仅一条含 diff 的块；头无 `[ok]`
- escalation_policy: 与 c1310 并行；若 End.result 形状变化，bridge 抽取路径跟 c1310 对齐

## Depends

- `c1280-update-app-tui-tool-chrome-quiet-defaults`（安静摘要基线）
- 软依赖：`c1310`（LLM result 瘦身；UI 仍可先吃现有 display_diff JSON）
