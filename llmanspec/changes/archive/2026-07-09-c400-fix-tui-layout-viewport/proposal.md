---
change_id: c400-fix-tui-layout-viewport
title: "fix TUI viewport anchoring — 底部输入/状态行在内容超终端高度时被推走（U1）"
status: draft
priority: 400
depends_on: []
author: agent
---

# c400-fix-tui-layout-viewport

## Why

c399 引入的单 Vec line-array 模型（同 pi）是正确的架构方向——pi 也用扁平数组，没有「sticky footer」概念，输入行/loader 只是 Component 树末端的更多行。基底模型无需改。

但用户反馈（经截图分析佐证，记录于 `_HANDOFF.md` U1）：对话内容超过终端高度后，底部输入区和 spinner 状态行被内容推走，与对话历史重叠、不可见。三轮 diff 管线修复（`5e11468`/`033139d`）解决了 scrollback 输出重叠/重复，但**视口锚定 bug** 仍然存在：viewport_top 在某些增量场景下落后于内容尾，导致底部固定行在视口外。

根因诊断：
1. 视口锚定在 diff 路径中依赖局部更新（scroll block + floor 纠偏），某些边界情况（内容刚好等于高度时新增行、 resize 后内容快速增长等）可能导致 viewport_top 一帧或多帧不追随尾部。
2. 缺少 scrollback 感知的集成测试——现有 `CapturingTerminal` 只记字节，不模拟终端实际滚动行为。c399 agent 自承这是测试盲区（turn 7 `033139d` 提交说明），且三轮手动验证都没测到 U1，因为没有自动化长流程覆盖。
3. 对 pi 的 `Math.max(height, newLines.length)` 做 working-area padding 的理解偏差——pi 始终以 `max(height, n)` 为逻辑总行数计算 `previousViewportTop`，避免短内容时 viewport 被顶部内容锚定而后续脱节。xylitol 的 full_render 用 `n - height`，diff_render 的 floor 用 `final_cursor - (height-1)`——近似但不严格等价，在某些增长模式中可能累积偏差。

**为什么 c400 排最高优先**：U1 是用户三轮手动验证都未解决的实证问题，阻塞 c356/c357（overlay/选择器需要正确布局）。已有的 diff 补丁边际收益递减，应在 c400 用**测试驱动的视口审计**一次性根治。

## What Changes

1. **视口锚定修正**：统一 `previous_viewport_top` 的设定逻辑为 `max(0, max(height, n_new) - height)`，对齐 pi 的 working-area padding 语义。删除 diff_render 中的 floor 纠偏公式，改为在 do_render 末尾集中锚定（视口始终追随内容尾）。
2. **scrollback 感知的集成测试 oracle**：扩展 `VirtualTerminal` 支持 scrollback buffer（真实模拟 `\r\n` 滚出顶部的行），编写端到端场景测试覆盖「内容从少于高度 → 超过高度 → streaming 增量 → 输入区始终可见」。
3. **Spec 更新**：modify `tui1`/`tui12` 明确 viewport 锚定不变量（「viewport_top MUST = max(0, max(height, total_lines) - height)」），modify `tui32`/`tui53` 操作步骤适配当前引擎实现描述（删去旧 ratatui 时代术语）。

## Capabilities

- `app-tui`

## Impact

- `src/app/tui/engine/tui.rs`：do_render / diff_render / full_render 的 viewport 锚定逻辑
- `src/app/tui/engine/virtual_terminal.rs`：加 scrollback buffer 模拟
- `src/app/tui/engine/tui.rs` 的测试：新增集成场景测
- `llmanspec/specs/app-tui/spec.toon`：modify tui1 / tui12 / tui32 / tui53
