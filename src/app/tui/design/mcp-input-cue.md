---
version: "alpha"
name: "mcp-input-cue"
description: "/mcp editor-slot panel for MCP list/ready/armed; optional short cue points to /mcp (not a long next-turn dump)."
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

# MCP 发现：`/mcp` 面板 + 可选短 cue

> Token：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 静图：[`playground/`](./playground/) 槽 **Mcp cue**（`?slot=mcp-cue`）。
> 合约：c1210（`atm17` / `ath27` / mcp7 armed 快照）。
> 下轮预告对照：[`pending-runtime.md`](./pending-runtime.md)。头卡：[`loaded-resources.md`](./loaded-resources.md)。
> 词汇：[`docs/architecture/TUI信息面与chrome词汇.md`](../../../../docs/architecture/TUI信息面与chrome词汇.md)。

## 产品意图（已拍 · c1210 同波）

多 MCP（10+）时，**禁止**把 server 名单塞进 status / 下轮预告右侧（空间不够、吵）。

| 面 | 角色 |
|---|---|
| **`/mcp`（主）** | 替换 **editor 槽**的只读（或轻管理）面板：列表、连接态、tools armed / 下轮是否已进请求 `tools` |
| **短 cue（辅）** | 可选 1 行 dim，**固定**文案：`mcp pending (see /mcp)`；**MUST NOT** 带分数计数或枚举 id |
| **头卡 mcp 行** | 启动摘要仍可；长对话滚走后 **不**依赖它作唯一发现面 |

长对话看不见 welcome 卡片 → 靠 **`/mcp` + 可选短 cue**，不是贴输入刷墙。

## `/mcp` 面板 MUST

1. **`/mcp`（无参）** 打开面板，**替换 editor 槽**（对齐 `/model` / resume）；**MUST NOT** 居中 overlay。
2. **任何 host 态可开**：idle / agent-busy / bang-busy / MCP connecting；Esc 关槽，**MUST NOT** 因开面板 abort agent（busy 时与其它 editor 槽一致：先关槽）。
3. **行字段**（至少）：`id` · 连接态（connecting / connected / failed）· **tools**（not armed / armed · 下轮请求会带）· 可选 tool 数。
4. **汇总行**：`configured N · connected K · armed?`；失败短诊断可感。
5. **Esc**：关槽；本波 **不**要求面板内启停 server（管理动作可后置）。
6. Slash 目录 / Usage 暴露 `/mcp`；可选别名 `/mcps` → 同面板。

## 短 cue（可选 · 合约钉）

| | |
|---|---|
| 文案 | **固定** `mcp pending (see /mcp)`（connecting / tools 未 armed 同一句）；全部 armed 后 **收起** |
| 落点候选 | busy 下轮预告位 **或** idle 破例 1 行 status（playground 对照） |
| MUST NOT | 分数计数（`1/2`）；枚举 server id；拼接 `Next turn: model \| mcp a·b·c…` |

## 两相位语义（面板列）

| 列 | 含义 |
|---|---|
| **ready / connected** | transport 已连上 |
| **armed** | 已 overlay 进 ToolSet，**下一轮** `run` 的 provider `tools` 含该 MCP 工具 |

## 静图芯片（playground）

- **`/mcp` 面板**：connecting / mixed / armed 列表态
- **短 cue**：nextturn 区引导 / idle 引导 / armed 收起
- **反例**：cue 里堆 10+ id（标 ✗）

## 非目标（本设计文）

- 面板内热重载 / 启停单 server（可后置）
- 往 transcript 插 MCP ready 假消息
