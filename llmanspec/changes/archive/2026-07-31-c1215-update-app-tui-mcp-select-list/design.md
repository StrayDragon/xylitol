# Design: c1215 /mcp SelectList + open perf

## UX

对齐 [`mcp-input-cue.md`](../../../src/app/tui/design/mcp-input-cue.md)：

```text
 MCP · configured N · connected K · armed A
> id   phase   armed|not armed   tools=n     ← SelectList reverse
  …
 Esc · Enter closes
```

| 键 | 行为 |
|---|---|
| ↑↓ | 移动焦点 |
| Enter | 关槽（本波；toggle 后置） |
| Esc | 关槽；不 abort agent |

## Perf

```text
OpenMcp:
  if host has cached LoadedResourcesSnapshot (from tick/settle/reload):
    mount_mcp_select(cache)   // sync, instant
  else:
    snap = await driver.loaded_resources_snapshot()
    cache = snap
    mount_mcp_select(snap)
  optional: background refresh snapshot → remount keep selection
```

Tick 路径已有 `refresh_loaded_resources` → 写入同一缓存。

## 实现落点

| | |
|---|---|
| `UiRoot` | `mcp_list: SelectList` + rows；替 `mcp_panel_lines` |
| `mcp_slot.rs` | mount from snap；`to_select_item` |
| `slot_input` | ↑↓ → list；Enter → close_slot |
| `effects/slash` OpenMcp | 缓存优先 |
| Host | `mcp_snap_cache: Option<LoadedResourcesSnapshot>`（或复用 root.loaded_resources） |

**注意**：`UiRoot.loaded_resources` 已在每次 refresh 更新——开 `/mcp` 可直接 `mount` 自 `root.loaded_resources`，**避免二次 await**，除非 `mcp_configured==0` 且怀疑过期（仍可用当前缓存）。

## 非目标

- session disable MCP
- 假 MCP ready 消息
