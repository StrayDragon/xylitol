# A2UI × xylitol：规范目标、组件分类与 ROI（2026-08）

> **范围**：评估「用 A2UI 约束 agent 产出固定人机组件、并做 `packages/xylitol-tui-a2ui`」是否值得。
> **一手来源**：[A2UI 官方介绍](https://a2ui.org/introduction/what-is-a2ui/)、[A2UI Protocol v1.0 Candidate](https://a2ui.org/specification/v1.0-a2ui/)（本地镜像：`../a2ui-tui-rs/crates/gallery/a2ui/specification/v1_0/`）、[`a2ui-tui-rs`](file:///home/l8ng/Projects/__straydragon__/a2ui-tui-rs) README、本仓 `ask` / `Web与TUI同源` / `xylitol-tui` 边界。
> **非目标**：不定实现计划、不改 live specs、不嵌入 ratatui 后端。

## 一句话结论

**方向有意义；立刻做完整 A2UI 渲染包 ROI 偏低。**
价值在「闭集 catalog + 双向 action 线」降低跨端（TUI/Web）重复，不在「复刻 basic catalog 全 18 控件 + 完整 surface/dataModel 运行时」。
xylitol 已有同哲学的窄实现（`ask` → `ChoicePrompt`）；应先把这条扩成**产品 catalog / wire 同源**，A2UI 作协议参考与可选未来对齐，而非短期 `xylitol-tui-a2ui` 满配。

---

## 1. A2UI 是什么 / 目标

| 断言 | 来源 |
|---|---|
| A2UI = **声明式 JSON 协议**，不是 UI 框架 | [What is A2UI?](https://a2ui.org/introduction/what-is-a2ui/) |
| Agent 只发「要哪些 catalog 组件」；客户端用**自有原生组件**渲染；无任意代码执行 | 同上 · Core Value |
| 三层分离：UI 结构 / data model / 客户端渲染 | 同上 · Design Principles |
| 传输无关（A2A / AG-UI / MCP / SSE / WS / REST） | [Protocol v1.0 · Transport](https://a2ui.org/specification/v1.0-a2ui/) |
| **Catalog 可换**：basic 只是参考；可自定 catalog 锁死视觉语言 | Protocol · Basic Catalog / custom catalogs |
| v1.0 = **Candidate**；官网标注生产优先考虑 **v0.9.1 Current** | [a2ui.org](https://a2ui.org/) |

### 协议核心消息（v1.0）

**Agent → Renderer：** `createSurface` · `updateComponents` · `updateDataModel` · `deleteSurface` · `callFunction` · `actionResponse`
**Renderer → Agent：** `action`（及可选 `functionResponse`）；可带 data model 元数据（`sendDataModel`）

设计取向（对 LLM）：

- 扁平 adjacency list（`id` + 子 `ComponentId`），易流式、易纠错
- `Dynamic*`：字面量 / JSON Pointer / 函数调用
- 客户端能力协商：`supportedCatalogIds` / inline catalogs

### 与「HTML iframe / 任意代码」对照

| 方案 | 安全 | 原生感 | 跨端 |
|---|---|---|---|
| HTML/JS iframe | 差 | 差 | 表面统一 |
| 自由 markdown 表单 | 中 | 中 | 各端各自解析 |
| **A2UI catalog 白名单** | 强（只渲染已实现类型） | 强（客户端主题） | 强（同一 JSON） |

---

## 2. `a2ui-tui-rs` 作参考的边界

本地仓库是 A2UI v1.0 的 Rust 实现：**`a2ui-base`（协议/模型/校验）+ 多后端**（默认 **ratatui** TUI，另有 Slint/egui/Bevy/Iced/Dioxus/GPUI）。

| 可借鉴 | 不宜直接嵌入 xylitol |
|---|---|
| JSON schema、basic/minimal catalog、message processor、校验策略 | ratatui `Frame`/`Widget` 渲染路径 |
| 组件分类与「媒体占位」经验 | 与 `xylitol-tui` 的 `Component`/`Vec<String>` ANSI 差分模型冲突 |
| Gallery 样例（表单 / chat bubble / modal） | 产品 chrome（Editor/Diff/Tree）不该进 agent catalog |

用户判断成立：**分析 schema 与设计即可；不能指望嵌入使用。**

---

## 3. Basic Catalog 组件分类（相对 coding-agent）

Basic catalog（18）：`Text` `Image` `Icon` `Video` `AudioPlayer` `Row` `Column` `List` `Card` `Tabs` `Modal` `Divider` `Button` `TextField` `CheckBox` `ChoicePicker` `Slider` `DateTimeInput`
客户端函数（14）：校验类 `required/regex/length/numeric/email/and/or/not` + 格式化 `format*` / `pluralize` / `openUrl`。

### 按 xylitol 产品场景分级

| 级 | 组件 | 理由 |
|---|---|---|
| **P0 已有/近邻** | `ChoicePicker`≈`ChoicePrompt`；`Text`；轻量 `Button`/`CheckBox`；`TextField`≈ Other 自由填 | 对齐现网 `ask`；澄清/决策主路径 |
| **P1 值得产品 catalog** | `Tabs`/`Modal`（Ask 已有 tabs/review 语义）；`Card`/`Column`/`Divider`/`List`（结构壳） | 多题问卷、确认框、结果卡；仍属「问人」而非通用 GUI |
| **P2 跨端才有意义** | `Slider` `DateTimeInput` `Icon` | TUI 交互成本高；Web 更自然；coding-agent 非刚需 |
| **P3 终端占位 / 可裁** | `Image` `Video` `AudioPlayer` | a2ui-tui 也多是占位；本仓已裁完整 Image encode（`xylitol-tui` AGENTS） |
| **禁止进 agent catalog** | 产品 `Editor` / `Diff` / `TreeSelector` / `SettingsList` / chrome 槽 | 属宿主壳，不是 LLM 可生成面 |

### 与 xylitol 现状对照

```text
① xylitol-tui 组件库          — 引擎 widget 闭集
② EditorSlot 脸               — 产品「editor 区放哪张脸」
③ ask JSON schema             — 今日唯一 agent 可调用的结构化 UI 合约
```

尚无「生成式 UI DSL」；**哲学已是 A2UI 子集（闭集 + 客户端渲染）**，只是未采用 A2UI envelope / surface / dataModel。

相关：`src/infra/tools/ask.rs` · `src/app/tui/design/ask.md` · `docs/research/ask-ui-ux-landscape-2026.md` · wire 已有未接线的 `AnswerQuestion`。

---

## 4. 与「Web 与 TUI 同源」的关系

[`docs/roadmaps/Web与TUI同源.md`](../roadmaps/Web与TUI同源.md) 钉的是：

- **同源 MUST**：`XyDriver` 命令与 `XyEvent` 闭集、会话/队列/中止、即时设置、公共动作语义
- **同貌不要求**：像素、控件皮肤

A2UI 解决的是另一层：**agent 产出的交互部件**跨渲染器同构。
两者互补，**不互相替代**：

| 层 | 今日 xylitol | A2UI 可补 |
|---|---|---|
| 会话生命周期 | `XyEvent` | 不该塞进 A2UI |
| 结构化问人 | `ask` tool + oneshot | catalog + `action` / `AnswerQuestion` 可统一 |
| 产品壳布局 | host layout | **不应**让 agent 生成整壳 |

误区：把「同源」理解成「Web 也要跑完整 A2UI basic gallery」。正确切法是 **小 xylitol catalog**（Ask/Confirm/…）两端各写原生渲染器。

---

## 5. ROI 分析

### 收益（真实）

1. **跨端减负**：TUI/Web（及未来 App）对「问人 / 确认 / 轻表单」共用 schema + action 名，避免三套 Ask。
2. **安全与可控**：catalog 白名单 = 比自由 HTML/markdown 表单更稳（与产品 Trust≠工具 popup 不冲突）。
3. **LLM 友好**：扁平组件列表 + JSON Schema 可塞进 tool/structured output（A2UI 设计目标）。
4. **生态可选对齐**：若未来接 A2A/AG-UI/MCP elicitation 富 UI，envelope 可互通。

### 成本（常被低估）

| 成本项 | 量级（粗估） | 说明 |
|---|---|---|
| 完整 MessageProcessor + Surface + DataModel + 绑定 | **大** | a2ui-base 级工作；与现有 `XyEvent`/ReAct 双状态机 |
| 18 组件 × xylitol-tui 映射 | **中–大** | 多数 P2/P3 对 coding-agent 无近利 |
| Catalog 协商 / inline catalog / 函数运行时 | **中** | v1.0 RPC（`callFunction`/`actionResponse`）更重 |
| 与 host 焦点/overlay/`EditorSlot` 抢输入 | **中** | 产品面已有严格 Esc/busy 规则 |
| 规范漂移（v0.9.1 Current vs v1.0 Candidate） | **持续** | 过早钉 v1.0 满配有风险 |
| 维护第二套「几乎是产品 Ask」 | **中** | 若与 `ask` 平行，易分叉语义 |

### 选项对比

| 选项 | 投入 | 近 6 月价值 | 跨端杠杆 | 建议 |
|---|---|---|---|---|
| **A. 满配 `xylitol-tui-a2ui`（A2UI 兼容渲染器）** | 很高 | 低（Web 未开闸） | 高但晚 | **不做** |
| **B. 薄协议包**：只 schema/processor，渲染另接 | 高 | 中 | 高 | 等有第二面再做 |
| **C. 产品 catalog（xylitol-ask 子集）+ wire `AnswerQuestion`** | 中 | **高** | 中→高 | **首选** |
| **D. 仅文档/参考**：继续读 A2UI，扩 `ask` schema | 低 | 中 | 低 | 过渡期可 |

### ROI 判断

- **现在做 A**：负 ROI——引擎栈不同（ratatui≠xylitol-tui）、Web 未交付、已有 Ask 覆盖主场景。
- **现在做 C**：正 ROI——兑现「固定一体数据流」的最小切片，对齐 `Web与TUI同源` 与已存在的 wire 缝。
- **A2UI 规范**：作为 **catalog 形状与消息语义的参考标准**（尤其 `ChoicePicker`/`action`/`sendDataModel`），不必立刻合规。

---

## 6. 若未来要 A2UI 兼容，推荐落点（非计划）

分层贴合本仓（勿把生成式 UI 塞进 `xylitol-tui` 引擎）：

```text
protocol/  — 可选：A2UI envelope DTO 或 xylitol-catalog 消息（wire / ports）
infra/     — LLM 产出校验、tool 投影到 catalog 消息
app/       — 各面 Renderer（TUI host / 未来 Web）映射到原生控件
packages/xylitol-tui — 只提供通用 widget；不拥有 A2UI 状态机
```

包名若坚持 `xylitol-tui-a2ui`，语义应是 **「TUI 面的 A2UI/catalog 适配器」**，依赖 `xylitol-tui`，**不是**第二套引擎；且宜在 **产品 catalog 稳定 + 第二渲染面启动** 后再建。

---

## 7. 来源索引

- https://a2ui.org/introduction/what-is-a2ui/
- https://a2ui.org/specification/v1.0-a2ui/
- https://a2ui.org/ （版本状态表）
- `/home/l8ng/Projects/__straydragon__/a2ui-tui-rs` README + `specification/v1_0/`
- 本仓：`docs/roadmaps/Web与TUI同源.md` · `src/app/tui/design/ask.md` · `packages/xylitol-tui/AGENTS.md` · `docs/research/ask-ui-ux-landscape-2026.md`
