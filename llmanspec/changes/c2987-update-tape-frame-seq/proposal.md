---
depends_on: []
title: 明确 tape 帧的 seq 语义
needs_specs_change: true
---

# 明确 tape 帧的 seq 语义

## Why

- tape 帧（`TextDelta` / `ThinkingDelta` / `MessageUpdate` / `ToolExecutionUpdate`）当前复用 `journal.max_seq()` 作为 `seq`。
- 同一批多个 tape 帧会共享同一个 seq；`seq` 不再唯一标识帧。
- spec `server-core` 明确 tape 不得写入 journal、不得消耗 seq；但 tape 帧仍携带 `seq` 字段。
- 当前 TUI 不做 seq 去重，所以无可见问题；未来若客户端按 `seq <= last_seq` 过滤（重连去重），会丢掉实时 tape 帧。

## What Changes

- 明确 tape 帧的 `seq` 语义（见 design）：
  - **A. 引入独立 tape 序号**：tape 帧带自己的单调序号（或改用 `frame_seq` 字段），与 journal seq 分离；
  - **B. 保留 journal seq 作为“事实位置”，并在协议/文档中明确客户端不得按 seq 去重 tape**；
  - **C. 让 tape 消耗 seq**（与 spec 冲突，不推荐）。
- 若改变帧形状，评估 `PROTOCOL_VERSION` bump 与客户端兼容。
- 补测试：重连/去重路径下 tape 帧不丢失；tape 与事实事件 seq 关系可断言。

## Capabilities

- `server-core`：事件 seq 与 tape 语义。
- `protocol-app`：下行帧字段与客户端去重约定（若改形状）。

## Impact

- `crates/xylitol-serve/src/occupancy/host.rs`：`push_tape` 的 seq 赋值。
- `crates/xylitol-protocol` / `xylitol-wire`：若新增 `frame_seq` 字段。
- 客户端（TUI/Print）去重逻辑。

## Out of Scope

- journal 容量与重放策略。
- `partial` 快照恢复（已有 `c2954`）。

## Further Notes

- 由 PR #30 review F10 记录。
- 这是**协议语义决策**：改形状会影响 `PROTOCOL_VERSION`。
