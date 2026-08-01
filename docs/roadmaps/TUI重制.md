# TUI 重制（设计不变量）

> **未兑现方向**：重置 **`UiEntry` 主条目**（LLM stream、工具、对话正文）的呈现，承接 pi 的交互优点，视觉耳目一新，并保持可复制 / 复制省 token。
> **不是**整壳重做（footer / 树 / slash 槽另案）；**不是**现行 MUST。
> 调研：[`../research/coding-agent-tui-design-landscape-2026.md`](../research/coding-agent-tui-design-landscape-2026.md) · 引擎：[`../research/xylitol-tui-capability-hooks-vs-landscape-2026.md`](../research/xylitol-tui-capability-hooks-vs-landscape-2026.md)。
> 并行减噪：[`TUI视觉与信息表达.md`](./TUI视觉与信息表达.md)；跨面：[`Web与TUI同源.md`](./Web与TUI同源.md)。

## 产品目标

| 要 | 不要 |
|---|---|
| 重置 User / Thinking / Tool / Assistant(/Bash/Diff…) **主视线** | 先大改 chrome 整壳再碰条目 |
| 承接 pi：**一行摘要可展开**、键位旁注、`Name path:range`、工具成败语义、无 `ASSISTANT>` 长标签 | 丢掉可扫读 / 可展开 / 人类可读工具行 |
| **rail 皮肤**：左边轨 + 条目上空行；无全局蓝绿洗底 | A/B 并列糊成一团；把实验塞进 SSOT 槽墙 |
| **短链路**：独立 remaster 页 →（必要时）MUST → 产品 | 静图→demo→包→产品四跳全走 |

## 设计不变量（候选）

### I1 · 范围 = UiEntry 主条目

- 真值类型：`UiEntry`（`bridge/model.rs`）：User、Assistant、Thinking、Tool、Diff、Bash、Compaction、ScrollNotice、Error。
- 重制焦点：**流式助手正文 + Thinking/Tool/Diff/Bash 块形态**；ScrollNotice/Error 保持短 dim，不抢主视线。
- 壳层可微调以衬托条目，**不**作为本方向主交付。

### I2 · 承接 pi 的体验优点

- 折叠默认详略得当；摘要行本身可读（非仅图标）。
- 工具主视线 `Read path:12-40`；成败用**边轨色**表达即可，不必整块洗底。
- `(Ctrl+T)` / `(Alt+E)` / `(Ctrl+O)` 正交旁注在 rich/compact 可见；raw 可藏。
- 用户短前缀；助手正文无角色大标签墙。

### I3 · 风格皮肤 `rail`（主方案）

- 左边轨区分 user / think / tool-ok|run|err / assistant。
- **禁止** `user-message-bg` / `tool-*-bg` 作为默认条目洗底。
- **每个条目上方 ≥1 空行**，保证边界与扫读分组。
- 实现上视为可切换的 **style skin**（prototype：`?style=rail|current`），不是第二套信息架构。

### I4 · 复制双出口

- **语义复制**为一等（源文本，非栅格）。
- 终端选区仍可用；装饰档位影响拖选污染。
- 复制默认省 token：无 chrome 旁注、无徽章墙。

### I5 · 装饰档位

- `raw | compact | rich`：影响旁注与工具正文密度。
- 独立页：`just open-uientry-remaster`。

### I6 · 耳目一新的边界

- **可改**：洗底 → 边轨；条目间距；密度。
- **慎改**：键位习惯族、`XyDriver`/`XyEvent`、Trust、会话树主导航。
- **跨面**：折叠/展开减噪与 Web 同源；TTY 选区可分叉。

## 精简交付链路（默认）

```text
playground/uientry-remaster.html（rail 皮肤；与 SSOT index 分离）
        ↓ 人眼选定
design/*.md 补丁（仅当升 MUST；可与接线同 PR）
        ↓
src/app/tui 接线 + harness
        ↓ 仅当缺通用原语
packages/xylitol-tui
```

| 步骤 | 何时需要 | 何时跳过 |
|---|---|---|
| remaster HTML（独立页） | **默认必经** | — |
| SSOT `index.html` | 整壳/槽定稿 | 纯 UiEntry 皮肤实验 |
| demo | 键位 / 窄宽 / 流式残影 | 纯皮肤已拍板 |
| 包升级 | 引擎缺口 | 只改产品 scrollback 绘制即可 |

## 分阶段

| 阶段 | 用户可感知结果 | 路径 |
|---|---|---|
| **M0 prototype** | `just open-uientry-remaster`：rail + 空行 + 无洗底 | 独立 HTML |
| **M1 条目呈现** | 产品主滚动区采用 rail（或等价） | 产品接线 |
| **M2 语义复制** | 复制出口无装饰墙 | 宿主为主 |
| **M3 引擎按需** | 仅证明确需时 | `package-tui-*` |

## BDD 意图示例

**场景：主条目不换键位习惯**
Given 默认 Emacs 编辑与 Alt+E / Ctrl+O
When 启用 rail 皮肤
Then 展开/视口/提交/中止路径不因视觉档位失效

**场景：无全局洗底**
Given 工具成功与用户消息同屏
When 用户扫读主滚动区
Then 条目以边轨与空行分组，不以整行蓝/绿 bg 铺底

**场景：语义复制干净**
Given 助手与工具块处于 rich 绘制
When 用户走语义复制
Then 剪贴板无键位旁注、无 footer、无角色徽章墙

## 相关

- Prototype：`src/app/tui/design/playground/uientry-remaster.html`
- SSOT：`src/app/tui/design/playground/index.html`
- 现行：[`../architecture/TUI信息面与chrome词汇.md`](../architecture/TUI信息面与chrome词汇.md) · `design/transcript.md` · `design/expandable.md`
- 索引：[README.md](./README.md)
