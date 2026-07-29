---
name: codebase-life-review
description: >-
  人类主动触发：仅当用户显式调用 /codebase-life-review 时启用。对已成长期、易失控的
  代码库做「人生回顾」式回归分析：宏观架构方向 × 重点逻辑脉络，产出临时理解构件
  （mermaid / 入口索引 / 张力点），默认写入 `_HANDOFF/` 或 `*.tmp.md`。Agent MUST NOT
  自动启用。Keywords: /codebase-life-review, 人生回顾, 逻辑脉络, 架构回看,
  life review, logic vein, reacquaint, 失控, 理解地图.
disable-model-invocation: true
---

# Codebase Life Review（人生回顾）

像重新审视前半生一样：不急着改造，先把**宣称的方向**与**代码事实**对上，再把几条
**重点逻辑脉络**画清楚，让人类（与本会话 agent）重新可控。

**仅人类主动调用**：唯一合法触发为用户显式输入 **`/codebase-life-review`**
（可带范围参数）。同义口述（「人生回顾」「逻辑脉络审视」）**不算**自动启用条件——
须用户补打 `/codebase-life-review` 或明确说「按 codebase-life-review skill 执行」。
Agent MUST NOT 因代码复杂、失控感、架构讨论而自行启用本 skill。

## 与相邻 skill 的边界

| Skill | 关系 |
|---|---|
| `llman-sdd-explore` | 动手前理清**新意图**；本 skill 回归**现状脉络** |
| `llman-sdd-wayfinder` | 回顾后发现「一团乱的大活」→ 交接 wayfinder |
| `llman-sdd-arch-review` | 回顾后想加深薄模块 → 交接 arch-review |
| `llman-sdd-propose` / `quick` | 回顾后要改 MUST/SHALL 或小修 → 交接对应路径 |
| `mermaid-expert` | 画/修 mermaid 时拉取 |
| `vscode-merfolk-expert` | **本版不默认落代码旁图**；未来若沉 `_vein/` 再拉 |

## 非目标（MUST）

- MUST NOT 自动改行为合约、批量重写 `AGENTS.md` / `docs/architecture/`
- MUST NOT 把临时理解升格为规范 SSOT（禁止新建第二套 architecture 百科）
- MUST NOT 默认在 `.rs` 嵌入 merfolk / rustdoc mermaid 宏（aquamarine / merman 等）；
  代码旁嵌入是**后续可选**，须人类另开确认
- MUST NOT 一次扫完全仓；默认 1 次回顾 = 宏观一页 + **1–3 条**脉络

## 真值源优先级

1. **代码**（类型、调用链、测试）— 最终事实
2. 最近层 `AGENTS.md` — 分层 / 开闭不变量
3. `docs/architecture/` — 产品心智（高维；不钉易腐路径）
4. `llmanspec/` live specs + archive design — 合约与设计史
5. `git log` 热点 — 辅助定范围，不替代代码

宣称与代码冲突时：**标张力点**，以代码为准叙述「现状」；产品文若过时，记入
「建议回写」清单，**不自动改**。

---

## 工作流

复制进度清单：

```text
Life Review Progress:
- [ ] 0. 定范围（宏观 / 脉络列表 / 深度）
- [ ] 1. 宏观回看（方向 × 分层 × 张力）
- [ ] 2. 选定 1–3 条逻辑脉络
- [ ] 3. 逐条走查（入口 → 主干 → 副作用 → 持久化）
- [ ] 4. 写出临时构件包 → `_HANDOFF/` 或 `*.tmp.md`
- [ ] 5. 与人类对齐：够用？加深？交接 SDD？
```

### 0) 定范围

**步进默认**：用户说「一步步来」时，一次只交付一个工作流步骤（0→1→2→…），
每步结束用结构化选项问是否进入下一步；**禁止**一轮写完整份回顾。
模板形态未定时：先薄后厚，落盘标 `template-draft`，后续轮次再收敛模板。

向人类确认（缺省则采用括号内默认）：

- **镜头**：整仓鸟瞰 / 一层（agent|infra|app|protocol）/ 一条产品能力
- **脉络数**：1–3（默认 2）
- **深度**：地图级（入口+图） / 游记级（关键函数级证据）
- **落盘名**：`_HANDOFF/life-review-<topic>-YYYYMMDD.md`（或用户指定）

用户若只给痛点词（如「compact 细节」），先映射成脉络候选再确认，勿直接开写。
未收到 `/codebase-life-review`（或等价明确授权）时：只回答普通问题，不跑本工作流。

### 1) 宏观回看

读：根与 `src` `AGENTS.md`、`docs/architecture/README.md` + 与镜头相关的 1–3 篇产品文；
对照实际目录树与公开 seam（如本仓 `XyDriver` / `XyEvent` / ports）。

产出一小节 **Macro**：

- 产品定调（是 / 不是）是否仍成立
- 分层依赖方向是否被代码遵守（抽 2–3 个反例或「未见反例」）
- **张力点**：文档宣称 vs 实现；理想 vs 现状（引用产品文「理想 vs 现状」若有）

可选一张 **layer flowchart**（粗、少节点）。

### 2) 选脉络

一条「逻辑脉络」= 用户可感知的一次完整因果链（或内部关键子系统闭环），不是模块清单。

命名用领域语言，例如：`一轮对话主循环`、`自动压缩触发与切点`、`会话树 leaf/fork`、
`JSONL 持久化读写`。

每条脉络开工前写三行：**用户碰到什么 / 入口 seam / 成功时结束在哪**。

### 3) 走查（每条脉络）

按固定探针，避免散文迷路：

| 探针 | 找什么 |
|---|---|
| Entry | 对外 API / port / 面如何进来 |
| Spine | 主干调用顺序（3–12 步） |
| State | 关键状态谁持有、何时变 |
| Side effects | 事件、hook、日志、磁盘 |
| Persistence | 读哪些、写哪些、格式/切点 |
| Config | 开关与默认值落点 |
| Tests | 哪类测试锁住这条链 |

证据：`rg` + 读关键文件；需要时用 `llman sdd context --task --paths` 挂合约。
画图前可拉 `mermaid-expert`。优先 `sequenceDiagram`（跨角色时序）或 `flowchart`
（闸门/分支）；状态机用 `stateDiagram-v2`。

### 4) 临时构件包

**默认落盘**（人类未禁止写文件时）：

- 目录：`_HANDOFF/`（若不存在则创建）
- 主文件：`life-review-<topic>-YYYYMMDD.md`
- 可选旁路：同 stem 的 `*.mmd`（复杂图外置，主 md 用 fenced mermaid 或相对链接）

文件头 MUST 声明：

```markdown
> TEMP / 非规范。人生回顾临时构件。勿升格 AGENTS / architecture / llmanspec。
> 过期可删。生成：codebase-life-review · <date> · 范围：…
```

主文件结构模板：

```markdown
# Life Review · <topic>

## Macro
- 定调 / 分层：…
- 张力点：…

## Vein · <name>
### 三行摘要
### 入口索引（路径 + 符号）
### 主干时序 / 流程图
### 配置与持久化要点
### 易迷路点
### 建议下一步（可选：explore / wayfinder / arch-review / propose）

## Open questions（留给人类）
```

会话内仍应用 mermaid 渲染关键图，方便当场讨论；落盘是为跨会话续看。

### 5) 对齐与交接

回顾结束时 MUST 问人类（结构化选项）：

- 构件是否够用 / 要加深哪条脉络
- 是否开 wayfinder / arch-review / SDD
- 是否**另行**讨论代码旁沉锚（merfolk `_vein/` 或未来 rustdoc）——本 skill 默认不做

---

## 构件类型速查

| 构件 | 何时用 |
|---|---|
| Layer flowchart | 宏观分层 / 依赖方向 |
| Sequence | 跨 UI·Driver·Agent·Store·Model |
| Flowchart + 闸门 | compact 触发、权限、trust |
| State diagram | leaf / fork / busy / abort |
| 入口索引表 | 文件路径 + 类型/函数名（可钉符号，少钉行号） |
| 张力点列表 | 文档 vs 代码；理想 vs 现状 |
| 配置键清单 | 只列本脉络真正读取的键 |

图要**小**：单图节点建议 ≤ 12；更大则拆「总览 + 细节」两张。

---

## 样例脉络（本仓痛点起手；skill 本身通用）

以下仅作 xylitol **候选**，其他仓库用同构方法自定名称与入口。

### A. 核心 agent 流程（一轮对话）

- 产品心智：`docs/architecture/一轮对话.md`、`插话续跑与中止.md`、`用户可见事件.md`
- 实现入口（走查起点，以代码为准）：
  - 面 → `XyDriver::run` / 队列 API（`src/app/core/driver/`）
  - 编排 → `AgentRuntime` / ReAct（`src/agent/session/`、`src/agent/runtime/react.rs`）
  - 投影 → session → LLM（`src/agent/llm_project.rs`）
  - 工具批 → `tool_batch` / `tool_exec`
- 探针重点：steer/follow-up/abort 是否只经 Driver；工具「意图收齐再批」；事件流 vs
  `XyEventSink` 侧路

### B. Compact 细节

- 产品心智：`docs/architecture/压缩与上下文.md`
- 实现入口：`src/agent/compaction/`（`orchestrator`、`settings`、`cut_detector`、
  `token_estimator`、`overflow`）；会话侧 `maybe_auto_compact` / `force_compact`
- 探针重点：reserve 公式与配置键；auto vs manual vs overflow；切点与
  `keepRecentTokens`；estimate provenance 与 footer 是否同源；生命周期事件

### C. Session 组织

- 产品心智：`docs/architecture/会话与持久化.md`
- 实现入口：`SessionTreeKind` / travel（Driver + protocol）；`AgentRuntime` 与 leaf；
  TUI resume（`src/app/tui/session_resume/`）
- 探针重点：message-history 树 vs file-browser；fork 切点；当前 leaf 语义；
  面如何 travel 而不碰 store 内部

### D. Persist · JSONL / 配置

- 产品心智：同上「会话与持久化」；配置心智见 `配置与档案.md`（若涉及路径/档案）
- 实现入口：`XySessionStore` port（`protocol/ports`）；`infra/session/`（manager 等）；
  export/import（agent session export + Driver `export_jsonl` / `import_jsonl`）
- 探针重点：条目类型与追加顺序；加载时 compaction cut；cwd 校验；配置如何选
  session 根目录 / 档案；导出格式与运行时 store 是否同一套

一次回顾建议：**宏观 + A**，或 **B+D**，或 **C+D**；勿四条一次做满。

---

## 执行纪律

- 先读后画；图上的每个节点应能指回符号或文件
- 少贴大段代码；入口索引 + 图 + 易迷路点 优先
- 发现应改合约/架构时：记入「建议下一步」，不在本 skill 内实施（除非人类明确
  退出回顾并指定 quick/propose）
- 画图语法问题 → `mermaid-expert`；勿臆造不稳定 mermaid 特性

## Done checklist

- [ ] 宏观有张力点或显式「未见张力」
- [ ] 每条脉络有三行摘要 + 入口索引 + ≥1 张图
- [ ] 临时文件头含 TEMP 声明；路径已告知人类
- [ ] 已用结构化选项询问：加深 / 交接 / 结束本次回顾
)
