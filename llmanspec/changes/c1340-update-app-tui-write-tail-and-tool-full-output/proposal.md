---
id: "c1340-update-app-tui-write-tail-and-tool-full-output"
stage: "draft"
depends_on:
  - "c1330-update-bash-full-output-align-pi"
  - "c1300-update-app-tui-write-edit-process-chrome"
---

# Proposal: write 流式尾视口 + 工具块 Full output TUI 提示

## Why

手工回归（长 write / 长 bash tool）：

1. **图1 write**：过程 chrome 用 `TruncateFrom::Head`，默认露出**文件首几行** + `N more lines`。流式写入时用户需要始终看到**流末尾**（对齐 bash 的 `earlier lines` / 跟 tail），否则像「卡在开头」。
2. **图2 bash 工具 + Alt+E**：Agent 侧已有截断与 `/tmp/xylitol-output-….txt`（context 脚注），但 Alt+E 展开后 TUI **缺少**可见的 `[Full output: …]` warning 脚注；超大 `output` 缓冲在展开路径上还会导致整块变卡（渲染/差分过重）。

c1330 已对齐 bang/executor 的 Full output 文案与 warning 色；本 change 收口 **write 视口方向** 与 **工具块（尤其 bash）Alt+E 路径的脚注可见性 + 展开上限**。

## What Changes（意向，draft）

1. write 正文 viewport：默认 **Tail**（跟 stream 末尾）；hint 形如 `… (N earlier, total, ctrl+o)`（或等价），与 bash 工具块一致。
2. Tool 块（bash 等）Alt+E 可见输出：若 result/history 已截断，MUST 展示 pi 形 Full output 脚注（warning）；MUST NOT 仅靠 agent 自然语言复述。
3. 展开路径（Ctrl+O / Alt+E 组合）：对超大 `output` MUST 有硬上限或保持 viewport，避免一次 paint 上万行卡死 TUI（具体策略 apply 时定：保留 footer + 尾视口，或展开仍封顶）。

## Capabilities（预计）

- `app-tui-transcript`（modify att14 / att15 或增补）
- 可选触及 `agent-tools` 仅当工具 End→UiEntry 需补 footer 字段

## Impact

- 改 write Head→Tail 会影响现有 att14「默认至多 10 行 Head」场景文案 → 同步 live specs。
- 非目标：改 50KB accumulator 阈值；OTel；非 TUI 的 inspect skill。

## 非本 change

- 上轮选项「本地肉眼再测」不纳入实现任务（用户已自测出图）。
