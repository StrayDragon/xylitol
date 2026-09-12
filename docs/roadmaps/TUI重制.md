# TUI 重制（设计不变量）

> **方向**：重置 **`UiEntry` 主条目**呈现（rail 皮肤）；承接 pi 交互优点，无默认洗底，可复制 / 复制省 token。
> 调研底稿已随 2026-09-12 cleanup 移除，结论吸收进 [`../../src/app/tui/DESIGN.md`](../../src/app/tui/DESIGN.md)。
> 并行减噪：[`TUI视觉与信息表达.md`](./TUI视觉与信息表达.md)；跨端：[`跨端同源.md`](./跨端同源.md)。
> 现状：主条目重制主体（rail 缺省、键位旁注、复制双出口）**已兑现**；本文只剩未兑现候选 **M4 悬停高亮区块**。

## 已兑现去向（事实只在一处写全，本文不复述）

| 已兑现 | 事实源 |
|---|---|
| rail 皮肤与条目呈现不变量（工具类左边轨 / 无全局洗底 / user 禁洗底） | [`../../src/app/tui/DESIGN.md`](../../src/app/tui/DESIGN.md)（视觉 SSOT）· [`../architecture/TUI信息呈现与固定区词汇.md`](../architecture/TUI信息呈现与固定区词汇.md) |
| 键位旁注 `(Ctrl+T)` / `(Alt+E)` / `(Ctrl+O)` | 产品键位表（`src/app/tui/keybindings.rs`，代码为真值） |
| 复制双出口（`/history-copy-last` + 应用内选区，复制默认省 token） | 选区词汇见 TUI信息呈现词汇（终端原生选区 / 应用内选区）；命令见产品 slash 表 |

## 边界（仍有效）

- **可改**：洗底 → 边轨；条目间距。
- **慎改**：键位习惯族、`XyDriver`/`XyEvent`、Trust、会话树。

## M4 悬停高亮区块（候选意向）

产品 TUI 引入 **鼠标悬停高亮语义区块**：hover 到工具块 / 折叠簇 / 队列条等区块时给出可点暗示（微弱底色 tint，非反色），点击就地展开/聚焦——参考 opencode session-v2 的 `BlockTool` 模式（语义块包一层带鼠标事件的容器，命中交给渲染层，无手工坐标表）。方向是**逐步替代「最小原则折叠小三角点击」**作为主交互；activity-fold 级别的折叠块（多行信封）仍需独立表达，不走逐块 hover。设计稿侧已在 `designing/app` shell 页演示同款交互（`data-region` + 事件委托），可作交互手感原型。

## 相关

- 对照稿：仓库顶层 `designing/` · `just open-designing`（代码为运行时 SSOT）；晋级/淘汰 SOP：[`designing/AGENTS.md`](../../designing/AGENTS.md)
- 现行：[`../architecture/TUI信息呈现与固定区词汇.md`](../architecture/TUI信息呈现与固定区词汇.md) · `designing/tui/modules/transcript` · `designing/tui/modules/expandable`
- 索引：[README.md](./README.md)
