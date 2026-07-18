---
id: c1340-update-app-tui-write-tail-and-tool-full-output
stage: full
depends_on:
- c1330-update-bash-full-output-align-pi
- c1300-update-app-tui-write-edit-process-chrome
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# Proposal: write 流式尾视口 + 超限 tool/bash 禁展开

## Why

1. **write** 默认 Head 露出文件首行，流式时不像「跟着写」；应对齐 bash 跟 **Tail**。
2. **bash/tool** 已硬截断（`[Full output: …]`）时 Alt+E/Ctrl+O 再展开会把巨量缓冲 paint 进 TUI（变卡），且与截断语义矛盾；write 短正文仍允许 Ctrl+O。

## What Changes

1. write 正文 viewport：`TruncateFrom::Tail` + earlier hint；Ctrl+O 仍可展开全文。
2. bang Bash / tool（bash 等）输出若含 `[Full output:`（系统硬截断）：MUST 保持尾视口；Ctrl+O MUST NOT 展开全文；hint 标明 expand disabled；脚注 warning 可见。
3. ToolExecutionEnd：截断时 MUST 用 result 的截断 `combined`/`stdout` 替换流式累积缓冲（禁止保留全量 stream 再挂脚注）。

## Capabilities

- `app-tui-transcript`（modify att14；modify/add att15/att16）

## Impact

- att14 场景文案 Head → Tail / earlier。
- 非目标：改 accumulator 50KB；禁 Alt+E 开关块本身；write 禁展开。
