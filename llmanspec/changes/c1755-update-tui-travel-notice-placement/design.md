# Design: c1755 trailing scrollback notices

## 问题

`rebuild_scrollback_from_travel` 在清空后 **先** `push` `history @`，再投影路径 → 跟底时长史下该行不可见。其它会话操作已用 `HostSession::push_system_note` 尾随。

## 方案（选定）

**拆开「投影」与「通知」：**

```text
rebuild_scrollback_from_travel
  → clear + path entries only（无 banner）

apply_session_tree_travel
  → rebuild → push_system_note(history @ …)

apply_*_fork / switched / debug
  → rebuild → 仅既有产品 note（forked/switched/…）
  → MUST NOT 再追加 history @
```

### 为何不「rebuild 末尾统一 append history @」

fork/resume 已有语义不同的尾随 note；若 rebuild 再 append `history @`，底栏双 System，违反已拍「去重」。

### demo

`agent_demo` travel：`transcript.clear` → 推 path 条目 → **最后** `push_message(System, history @ …)`（今日是 System 在 path 前）。

### 非方案

- queue strip / status 槽：瞬时可滚回顾要落 scrollback；chrome 不替代。
- 改 `history @` 文案：另案。

## 测试边界（seam）

| seam | 覆盖 |
|---|---|
| `HostSession` + ScriptedDriver travel | 产品：重建后末条含 `history @`；首条不是 |
| `apply_session_tree_fork` | 有 `forked →`；无多余顶/底 `history @` |
| `agent_demo` travel 键序列 | 视口/transcript 含 `history @` 且不在仅顶插位置依赖 |

## 与 c1760

通知贴底后，activity-fold 「System 始终外显」才有意义。`blocks` / `depends_on` 边保持。
