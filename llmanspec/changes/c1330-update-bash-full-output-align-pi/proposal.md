---
id: c1330-update-bash-full-output-align-pi
stage: draft
depends_on:
- c1310-update-infra-tool-result-quiet-align-pi
- c1320-update-app-tui-tool-path-stream-chrome
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# Proposal: 对齐 pi 的 bash Full output（context + TUI）

## Why

长 bash 输出需要同时满足：

1. **模型 context**：写入截断尾部 + `[Full output: <path>. Truncated: …]` 指针，**禁止**把完整 stdout/stderr 塞进 tool JSON。
2. **TUI**：可见黄条 Full output 行（warning 色），与 viewport/`ctrl+o` 并存。

今日半齐：有 `OutputAccumulator` + `full_output_path`，但 footer 文案不像 pi；非流式 bash 工具结果仍可能带全量 `stdout`/`stderr`；TUI bang 只追加 `(truncated)`。

## What Changes

1. `OutputSnapshot::display_content` → pi 形 footer（含 path、shown lines、50KB limit）。
2. `XyBashResult.output` / bash 工具 JSON：截断正文 + footer；截断时 MUST NOT 再附完整 stdout/stderr。
3. 产品 TUI：Bash/Tool 输出中以 `[Full output:` 开头的行用 `{colors.warning}`（可 bold）绘制。

## Capabilities

- `infra-bash`（modify be3）
- `agent-tools`（modify t15 / 增补）
- `app-tui-transcript`（增补 Full output chrome）

## Impact

- BDD：`bash-truncate` / accumulator 场景期望含 `Full output` 或 `truncated` + path。
- 非目标：改滚动 viewport 行数；OTel；非 bash 工具的通用 sidecar。
