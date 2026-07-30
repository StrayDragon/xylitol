---
version: "1.0"
name: "mcp-input-cue"
description: "Shipped c1210: /mcp editor-slot panel + fixed short cue mcp pending (see /mcp)."
tokens_from: "../DESIGN.md"
components:
  mcp-panel-title:
    textColor: "{colors.muted}"
  mcp-panel-row:
    textColor: "{colors.on-surface}"
  mcp-panel-armed:
    textColor: "{colors.success}"
  mcp-panel-pending:
    textColor: "{colors.muted}"
  mcp-short-cue:
    textColor: "{colors.muted}"
---

# MCP 发现：`/mcp` 面板 + 短 cue（已落地 · c1210）

> Token：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 静图：[`playground/`](./playground/) 槽 **Mcp cue**（`?slot=mcp-cue`）——与产品行文对齐。
> 合约：`atm17` / `ath27` / mcp7；归档 `archive/2026-07-31-c1210-update-mcp-hot-merge-ungate/`。
> 实现：`layout/root/mcp_slot.rs` · `LoadedResourcesSnapshot` / `MCP_PENDING_CUE` · `refresh_mcp_short_cue`。
> 下轮预告对照：[`pending-runtime.md`](./pending-runtime.md)。头卡：[`loaded-resources.md`](./loaded-resources.md)。

## 产品意图

多 MCP 时 **禁止**把 server 名单塞进 status / 下轮预告。发现面：

| 面 | 角色 |
|---|---|
| **`/mcp`（主）** | 替换 **editor 槽**的只读面板：汇总 + 每 server 连接态 / armed / tool 数 |
| **短 cue（辅）** | 固定 `mcp pending (see /mcp)`（`MCP_PENDING_CUE`）；全部 armed 后收起 |
| **头卡 mcp 行** | 启动摘要；长对话滚走后不依赖它作唯一发现面 |

## `/mcp` 面板（as-built）

| | |
|---|---|
| 命令 | 无参 `/mcp` 或 `/mcps` → `PendingSlash::OpenMcp`；BusySlashPolicy **Allow** |
| 槽 | `EditorSlot::Mcp`；**MUST NOT** 居中 overlay |
| 数据 | `Driver::loaded_resources_snapshot()` → `mcp_servers: Vec<McpServerSnapshot>` |
| 任意态 | idle / agent-busy / bang-busy / MCP connecting 均可开；Esc 关槽，**不**因开面板 abort agent |

### 行文（与 `mount_mcp_panel` 一致）

```text
 configured N · connected K · armed A
 {id}  {connecting|connected|failed}  {armed|not armed}  tools={n}
 diag: {short…}          # 若有 mcp_diag_short
 Esc to close
```

无配置时第二行：`(no MCP servers configured)`。

### 相位 / armed

| 字段 | 含义 |
|---|---|
| `phase` | `Connecting` / `Connected` / `Failed` |
| `tools_armed` | ToolSet 已含至少一枚 `mcp:{id}:…` → **下一轮** provider `tools` 会带 |

## 短 cue（as-built）

| | |
|---|---|
| 文案 | **仅** `mcp pending (see /mcp)` |
| 何时显示 | `LoadedResourcesSnapshot::mcp_tools_pending()`（已配置且未全部 armed，或仍有 connecting label） |
| 落点 | `status_next_turn_cue`（busy 行右侧 **或** idle 破例 1 行 status） |
| 优先级 | busy 且已有 `Next turn…` 模型预告时 **不**覆盖（模型 pending 优先） |
| 收起 | 不再 pending 且当前 cue 恰为 `MCP_PENDING_CUE` → `None` |
| MUST NOT | 分数计数；枚举 server id；拼进 `Next turn: model \| mcp a·b·c…` |

## 静图芯片（playground · 对齐实现）

- `/mcp`：connecting / mixed / all armed（行文同上）
- 短 cue：busy / idle / Next turn 优先 / armed 收起

## 非目标（仍后置）

- 面板内启停 / 单 server reload
- transcript 假「MCP ready」消息
