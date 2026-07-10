# Design — c450 app-tui 合约修订

## 背景

旧 `app-tui` 合约描述的是已删除的 in-tree ratatui 引擎。真值源现为：

- 引擎：`packages/xylitol-tui`（host 驱动、`Vec<String>` ANSI）
- 视觉：`src/app/tui/DESIGN.md`（及即将拆分的 `design/*`）
- Seam：`Driver` / `dispatch` / `protocol::Command` / `XyEvent`

## 决策

### D1 — 能力拆分命名

| Capability | 职责 |
|---|---|
| `app-tui-host` | CLI 入口、host 事件循环、终端生命周期、即时 log |
| `app-tui-bridge` | XyEvent→UI 模型、流/队列生命周期、InProcess 默认 + Remote 预留 |
| `app-tui-transcript` | 对话呈现、可展开、diff 块 |
| `app-tui-chrome` | theme token、glyph、status、footer |
| `app-tui-input` | Editor 槽、键位、补全 source 接线 |
| `app-tui-commands` | slash ↔ `protocol::Command` |

旧单体 `app-tui`：本变更后仅保留**跨切面不变量**（reuse-contract、三面共存、无 skeleton spray），或缩成索引；细节 req 迁出。

### D2 — 与旧 req 的处置原则

| 旧主题 | 处置 |
|---|---|
| ratatui / Viewport::Inline / scrolling-regions / TestBackend | **remove** |
| `StyledLine` 作为引擎类型 | **remove**（引擎为 `Vec<String>` ANSI） |
| idle 常驻 Ready status（tui53） | **rewrite**：idle 0 行 |
| line-array / viewport 锚定 / hard width | **keep 语义**，改写为 xylitol-tui 引擎行为 |
| Driver / reuse-contract / 三面共存 / no dead skeleton | **keep** |
| slash /exit /model | **keep**，迁 `app-tui-commands` |
| markdown 细则 | **迁** transcript；实现依赖包 Markdown + 应用 highlight 回调 |
| 流 drain / 多 TurnEnd | **rewrite** 于 bridge：对齐 pi（见 D4），不照搬已删实现的临时结论 |

### D3 — 产品交互（已与用户锁定）

```
Esc（流中）           → Driver::abort
Ctrl+C（有输入）      → 清空编辑器
Ctrl+C（空编辑器）    → 退出（副作用：进程结束 + restore）
流中 Enter            → steer（下一 ReAct 迭代注入）
Alt+Enter             → follow-up（当前 Agent loop 全部完成后再跑）
双 Esc（空编辑器）    → 会话树（产品 c491；demo c456 先验证）
信任目录后            → yolo 执行工具；无逐工具审批 UI；hooks 可扩展
```

### D4 — 流生命周期（重读 pi，不沿用旧 TUI）

- 一次用户可见「轮」对应一次 `Driver::run`（或 steer/follow-up 触发的续跑）直至 **AgentEnd** 且 follow-up 队列空。
- 中间 **TurnEnd** 只表示 ReAct 单次迭代边界（常在工具调用后），**不得**据此清空 transcript 尾或复位 idle。
- **steer**：不取消当前迭代；在迭代边界注入用户消息。
- **follow-up**：AgentEnd 后再启动。
- UI busy：从提交/steer 起至 AgentEnd 且无待处理 follow-up。

（实现落在 c461/c465；本变更只把合约从「旧 tui52 字面」改为上述原则。）

### D5 — 审批与沙盒

不模仿 Claude Code / Codex 的 session 内逐工具确认。目录 **trust** 后默认执行；沙盒交给外部环境。TUI 保留 hook / 日后扩展点，不做审批面板 MVP。

### D6 — Feature

合约要求：`tui` 为 default feature 的一部分，避免本地/CI 漏编。Cargo 改动可与 c460 同落地，但 MUST 写入本变更 spec。

### D7 — 远控

默认 InProcess；`RemoteDriver` **保留**，供日后 server 远程 + TUI/web 控制。

## 权衡

- **为何先改合约再写代码**：前三次失败的共同模式是实现领先于稳定边界。
- **为何 purpose-draft 并行**：防失忆；升格前允许漂移受控在 draft 层。
- **specs 中文**：降低中英混写导致的 agent 误读；技术标识符保留英文。
