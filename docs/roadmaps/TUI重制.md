# TUI 重制（设计不变量）

> **方向**：重置 **`UiEntry` 主条目**呈现（rail 皮肤）；承接 pi 交互优点，无默认洗底，可复制 / 复制省 token。
> **M1 已兑现（c1830 tui-entry-rail-default）**：产品 scrollback；交互设计稿现为仓库顶层 `designing/`。
> 调研：[`../research/coding-agent-tui-design-landscape-2026.md`](../research/coding-agent-tui-design-landscape-2026.md) · 引擎：[`../research/xylitol-tui-capability-hooks-vs-landscape-2026.md`](../research/xylitol-tui-capability-hooks-vs-landscape-2026.md)。
> 并行减噪：[`TUI视觉与信息表达.md`](./TUI视觉与信息表达.md)；跨面：[`Web与TUI同源.md`](./Web与TUI同源.md)。

## 产品目标

| 要 | 不要 |
|---|---|
| 重置 User / Thinking / Tool / Assistant(/Bash/Diff…) **主视线** | 先大改固定区整壳再碰条目 |
| 承接 pi：**一行摘要可展开**、键位旁注、`Name path:range`、工具成败语义、无 `ASSISTANT>` 长标签 | 丢掉可扫读 / 可展开 / 人类可读工具行 |
| **rail**：工具类左边轨 + 条目空行；无全局蓝绿洗底 | 平行实验页与 SSOT 双源 |

## 设计不变量

### I1 · 范围 = UiEntry 主条目

- 真值类型：`UiEntry`（`bridge/model.rs`）。
- 重制焦点：流式助手正文 + Thinking/Tool/Diff/Bash 块形态；ScrollNotice/Error 短 dim。

### I2 · 承接 pi 的体验优点

- 折叠默认详略得当；摘要行可读。
- 工具主视线 `Read path:12-40`；成败用**边轨色**。
- `(Ctrl+T)` / `(Alt+E)` / `(Ctrl+O)` 正交旁注。
- 用户短前缀；助手无角色大标签墙；**thinking flush**（无轨）。

### I3 · 风格皮肤 `rail`（产品默认）

- tool / bash / diff：status 左边轨 + gutter。
- user / assistant / thinking：**flush**（无轨、无 `user-message-bg` / `tool-*-bg` 洗底）。
- 工具类条目上方 ≥1 空行。

### I4 · 复制双出口

- 语义复制为一等（源文本）；终端选区仍可用。
- 复制默认省 token。

### I5 · 耳目一新的边界

- **可改**：洗底 → 边轨；条目间距。
- **慎改**：键位习惯族、`XyDriver`/`XyEvent`、Trust、会话树。

## 交付链路（现行）

```text
designing/tui/modules（交互设计稿，辅助）
        ↓
src/app/tui 接线（运行时真值；已落地 rail）
        ↓ 仅当缺通用原语
packages/xylitol-tui（paint_left_rail_line）
```

交互手感（可选包演示，**≠** 产品静图 SSOT）：`just demo-tui` / `just demo-tui-rail`。真产品：`cargo run` / 产品 TUI。

## 分阶段

> M0（产品 rail）与 M1（index SSOT）已落地，以 [`designing/tui/modules/transcript`](../../designing/tui/modules/transcript/intent.md) 与代码为准；本文只留未兑现切片。

| 阶段 | 状态 |
|---|---|
| **M2 语义复制** | 候选 |
| **M3 引擎按需** | 候选 |
| **M4 悬停高亮区块** | 候选（c2550 后新意向，见下） |

## M4 悬停高亮区块（候选意向）

产品 TUI 引入 **鼠标悬停高亮语义区块**：hover 到工具块 / 折叠簇 / 队列条等区块时给出可点暗示（微弱底色 tint，非反色），点击就地展开/聚焦——参考 opencode session-v2 的 `BlockTool` 模式（语义块包一层带鼠标事件的容器，命中交给渲染层，无手工坐标表）。方向是**逐步替代「最小原则折叠小三角点击」**作为主交互；activity-fold 级别的折叠块（多行信封）仍需独立表达，不走逐块 hover。设计稿侧已在 `designing/app` shell 页演示同款交互（`data-region` + 事件委托），可作交互手感原型。

## 相关

- 对照稿：仓库顶层 `designing/` · `just open-designing`（代码为运行时 SSOT）
- 现行：[`../architecture/TUI信息面与固定区词汇.md`](../architecture/TUI信息面与固定区词汇.md) · `designing/tui/modules/transcript` · `designing/tui/modules/expandable`
- 索引：[README.md](./README.md)
