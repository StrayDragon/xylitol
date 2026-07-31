---
version: "1.1"
name: "mcp-input-cue"
description: "/mcp editor-slot SelectList (like /model|/resume) + fixed short cue mcp pending (see /mcp)."
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
  mcp-panel-selected:
    textColor: "{colors.on-surface}"
  mcp-short-cue:
    textColor: "{colors.muted}"
---

# MCP 发现：`/mcp` SelectList + 短 cue

> Token：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 静图：[`playground/`](./playground/) 槽 **Mcp**（`?slot=mcp-cue`）。
> 已归档：c1210（只读行文 + 短 cue + hot-merge ungate）。
> **下一刀定稿**：SelectList 壳（对齐 `/model` / `/session-resume`）+ 开面板 perf；合约拟 `c1215-update-app-tui-mcp-select-list`。
> 对照：[`models-picker.md`](./models-picker.md) · [`session-resume.md`](./session-resume.md) · [`pending-runtime.md`](./pending-runtime.md) · [`loaded-resources.md`](./loaded-resources.md)。

## 产品意图

多 MCP 时 **禁止**把 server 名单塞进 status / 下轮预告。

| 面 | 角色 |
|---|---|
| **`/mcp`（主）** | 替换 **editor 槽**的 **SelectList**：↑↓ 选中；行 = id · phase · armed · tools；为日后「本 session 临时关某 MCP」留 Enter 动作缝 |
| **短 cue（辅）** | 固定右对齐 `mcp pending (see /mcp)` |
| **头卡 mcp 行** | 启动 `connecting i/n` / connected 摘要；**不是**唯一发现面 |

## `/mcp` 面板 MUST（定稿 · SelectList）

1. **`/mcp` / `/mcps`（无参）** → 打开替换 editor 槽的列表；**MUST NOT** 居中 overlay。
2. **组件**：包 `SelectList`（或与 resume/models 同族）；**↑↓** 移动焦点；选中行 **reverse**（对齐 models/resume）；**Esc** 关槽且 **MUST NOT** 仅因开面板 abort agent。
3. **任意态可开**：idle / agent-busy / bang-busy / MCP connecting；BusySlashPolicy **Allow**。
4. **行字段**（至少）：`id` · `connecting|connected|failed` · `armed|not armed` · `tools=N`。
5. **汇总**：标题或首行 muted：`configured N · connected K · armed A`；`mcp_diag_short` 可感（标题下或底栏）。
6. **Enter（本波）**：MUST 有明确行为——**MVP = 关槽**（与「先可选项、后动作」一致）或短提示「toggle not available yet」二选一，静图用 **Enter closes**；**MUST NOT** 假实现关 MCP。
7. **数据**：经 Driver 只读缝（`LoadedResourcesSnapshot` / `mcp_servers`）；**MUST NOT** app/tui → infra::mcp。
8. **Perf**：开槽 MUST 优先用已缓存的最近 snap（host tick / settle 已刷）；**MUST NOT** 每次 `/mcp` 都阻塞等待完整 reconnect 式 discover；缓存缺失时再 await 一次 snapshot。

### 行文（SelectList item 意向）

```text
 MCP · configured 2 · connected 1 · armed 1
> context7   connected   armed      tools=2     ← 焦点 reverse
  lspz       connecting  not armed  tools=0
 Esc · Enter closes
```

### 相位 / armed

| 字段 | 含义 |
|---|---|
| phase | transport / 配置态：`Connecting` / `Connected` / `Failed` |
| armed | ToolSet 已含 `mcp:{id}:…` → 下轮 provider `tools` 会带 |

## 短 cue（已落地 · 保持）

| | |
|---|---|
| 文案 | **仅** `mcp pending (see /mcp)` |
| 落点 | status **右对齐**（idle 整行 / busy 贴 Working\|Drafting 右侧） |
| 何时 | `mcp_tools_pending()`；可与头卡 `connecting i/n` 并存 |
| 优先级 | busy 且已有 `Next turn…` 时不覆盖 |
| 收起 | 全部 armed / 不再 pending |

## 静图芯片（playground）

- `/mcp` SelectList：connecting / mixed / all armed（含焦点 reverse）
- 短 cue：idle/busy 右对齐 · Next turn 优先 · armed 收起

## 非目标（后置 change）

- 本 session 临时禁用 / 启停单 server（Enter 真动作）
- 面板内 `/reload` 单 server
- transcript 假「MCP ready」
