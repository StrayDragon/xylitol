# Design — c730-refactor-app-tui-host-modules

## 模块目标形状

```text
host/
  mod.rs          — HostSession 类型 + step 调度
  pending.rs      — PendingOps
  input_policy.rs — busy/idle Esc、steer、bang 拒绝
  mounts.rs       — tree/models mount helpers
layout/
  root.rs         — 薄协调
  tree_slot.rs / models_slot.rs / editor_route.rs  — 按需
commands/         — 已有；可拆 parse vs pending types
effects.rs        — 唯一 drain_pending（不动为第二泵）
```

不必一次拆到文件级完美；**PendingOps + input_policy 抽出**为 MUST；UiRoot 按痛点拆。

## busy-bang（ati32）

| 状态 | `!cmd` Enter |
|---|---|
| idle | execute_bash（既有） |
| bash_active | 硬拒（ati20） |
| agent busy | **硬拒（本变更）** — 不再 steer 字面 `!cmd` |

## get_messages

travel/fork/label：`Err` → `push_system_note`；成功才 rebuild scrollback。

## 明确不做

- 合并 Host/UiRoot 双 UiModel（future）
- 把 c716 文档内容复制进代码注释长文
