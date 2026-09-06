# llmanspec AGENTS.md

此文件由根目录的 `AGENTS.md` 托管块引用，存放 **llman SDD 工作流在本项目的专属规则**：
change/spec 的命名、ID、依赖、原子性、语言。架构事实（分层、seam、Xy\*）真源在
`src/AGENTS.md`，不在此重复；BDD 配置真源在 `llmanspec/config.yaml` 的 `bdd:` 段。

> 写作原则：只写稳定规则与硬约束（有 specs/changes 代码事实支撑）。进度、当前 active
> 清单、行数等易腐内容不写——查 `llman sdd list` / `llman sdd status`。

## Artifact 规则

### proposal

- **Change ID 格式**：`c{priority}-{verb}-{subject}`。
  - `verb` ∈ `add` / `update` / `remove` / `refactor` / `fix`（实测无其它）。
  - `priority` 为整数。**建议**用 5 的倍数（c05/c10/c1200）——目的是**预留加塞间隙**，方便后续插队；**非**强制 5 倍数。需要插队时直接占用相邻空位。priority **MUST 唯一**。
- **priority 是建议性排序**：实际执行先沿 `depends_on` 依赖边，priority 仅作并列时的 tiebreaker。
- **frontmatter**：每个 `proposal.md` MUST 含 YAML frontmatter，至少带 `depends_on`（list，无依赖用 `[]`）。
- **依赖闸**：`depends_on` 引用的 change 全部归档（移入 `changes/archive/`）前，本 change 不可 apply。引用不存在的 change = 校验错误，STOP。
- **原子性**：每个 change 独立可校验、可归档。

### spec（capability 命名）

- spec 目录名 = **capability**：领域名词、kebab-case，描述 **WHAT**（不写动作）。
  例：`runtime-config`（非 add-config）、`agent-runtime`（非 add-agent-loop）、`tool-system`。
- **capability 前缀按层**（实测分布，normative）：
  - `package-tui-*` — `packages/xylitol-tui`
  - `package-ai-bridge*` — `packages/xylitol-ai-bridge`
  - `app-tui-*` — 产品 TUI（`src/app/tui`）；可多 capability，禁止把新产品合约堆进单体 `app-tui`
  - `agent-*` / `domain-*` / `runtime-*` / `infra-*` / `protocol-*` / `server-*` / `cli-*` — 对应分层
  - `test-*` — 测试基础设施（BDD harness、fake provider、qa-gate…）
  - `workspace-*` / `build-*` / `layer-*` / `user-*` / `architecture` — 仓库 meta / 跨层
  - 历史名可并存。

### tasks

- 单 task 拆分上限约 2 小时。
- purpose-draft change 可只交付 `proposal.md`（`status: purpose-draft`）；apply 前须 promote 为完整工件（specs + tasks）。

### research（change 附属调研）

- **临时 / change 作用域调研**（选型对照、一手摘录、仅对本 change propose/design 有意义、易随决策过期）MUST 放在 `llmanspec/changes/<id>/research/<topic>.md`（随 change 进 `archive/`）；**禁止**为此类笔记新开或堆进 `docs/research/`。
- 产出是 Change 文档，**不是** live specs；关键结论 SHOULD 摘要回写 `proposal.md`「Further Notes」（附文件指针）。
- `docs/research/` **仅**留给跨多个 change / 归档后仍常引用的耐久底稿（主题级、非单次选型备忘）。升格条件：六个月后仍被多条主线引用，且不是单 change 决策草稿纸。

## 语言

- spec 的 `purpose` / requirement statement / scenario 步骤 **MUST 中文**；技术标识符（类型名、路径、命令、req_id）保留英文。规则块场景名用 requirement title；验收场景名用英文 `scenario.id`。
- **单轨 feature-as-spec（r131）**：每个 capability 恰好一个 live spec 文件 `llmanspec/specs/<capability>/<capability>.feature`；`spec.toon` 已退役，validate 拒绝读取。文件头部注释 `# language:` / `# capability:` / `# purpose:` / `# scope:` 必备。
- 场景三档（标签决定语义）：
  - `@req:<id> @human` = **约束规则**（statement 须含 MUST/SHALL/必须/不得/禁止）；已锁定（r135），agent 修改须走 ack 流程；
  - `@executable`（+ `@req:<id>`）= 验收场景，由 `tests/bdd/bindings_*.rs` 的 `#[scenario(path=…, name=…)]` 按**精确名与步骤文本**绑定；`@req` MUST 指向本文件已定义的规则；
  - `@human @manual` = 人工豁免。
- `背景:`（Background）MUST 紧跟 `功能:` 行（中间不得有空行）——rstest-bdd 才会执行其步骤。
- 在非默认 feature 分支直接编辑 live `.feature` → `llman sdd change attach` / `checkpoint` → docs-only `change archive` → Git merge。**禁止** `solidify`、`change delta`、新建 `*.feature.delta.toon`。与 `tests/features/` 手写链路可并存。

## spec 约束层级（产品级优先）

- requirement statement MUST 描述**产品可观察行为 / 数据契约**（WHAT），中文；**禁止硬约束代码组织**：具体路径、文件/模块名、类型名、行数、方法归属、迁移清单。
- 例外——**大的组织方向**可保留：分层依赖方向、端口 seam、crate 边界、组合根职责、跨面同源（如产品 slash SSOT）。
- 代码组织演进（重构、改名、移动）不要求改 spec；spec 只随产品行为变化而变。
- 已删除对象（类型/模块/方法）的引用条款随删除一并清理，不保留「防复活」清单（除非有真实回归风险）。

## spec 维护（产品级同步，直接编辑）

- 因代码组织演进导致 spec 过期（主语/路径/迁移条款）→ **直接编辑** live `.feature` 并直接 commit，**免 change 生命周期**（无需 attach/checkpoint/finalize/archive）。
- 新增/变更**产品行为**仍走标准 change 流程（propose → apply → verify → archive）。
- 直接编辑仍 MUST 过结构门禁：`llman sdd validate <cap>`（或 `--all`，BDD-on 下含 runner check）与相关 BDD 测试绿。
- 删除 req 时同步清理：`.feature` 的规则块与 `@req:` 验收场景、`tests/bdd` 的 scenario binding。

## change 操作闸

### checkpoint / finalize 提交序（MUST 知悉）

**推荐（单 commit）**：

```text
实现 live specs + 代码（工作区可脏）
→ llman sdd change finalize <id> [--no-check]
→ git commit   # 一次：实现 + frontmatter + archive 改名
```

- `finalize` **不要求**干净树；写入 `checkpointed: true` 且 `checkpoint_sha = attach 时 base_sha`（不是实现 HEAD）。
- 审计仍可用：`git diff base_sha..HEAD` + `branch`。

**Fallback（多 commit，严格 sha）**：

```text
commit（live specs + 代码）
→ llman sdd change checkpoint <id>   # checkpoint_sha = 实现 HEAD
→ commit checkpoint 元数据
→ llman sdd change archive <id>
→ commit archive rename
```

- 结构门禁先跑：`llman sdd validate <cap|change> --strict --no-check`（快）；再跑带 BDD 的全量 validate / finalize。
- `checkpoint`/`finalize`/`archive` 的 `--no-interactive`：接受并忽略。

### 提交卫生（SHOULD）

1. **Draft 可独提或一批提**：可从 `docs/roadmaps` 等意向一次 `change new` 多个草案并 `chore(sdd): draft …` 入库；**不**要求与实现同提。
2. **闭环收尾优先 `finalize`**，减少 checkpoint/archive 礼仪 commit。
3. **产品 vs 流程**：实现用 `feat`/`fix`/`refactor`；SDD 礼仪用 `chore(sdd):` / `docs(sdd):`。

### stage=draft

已有 `proposal+design+tasks` 仍报 `draft` 时：通常是 **未 attach** → `llman sdd change attach <id>`（不要新建 `changes/<id>/specs/`）。attach 后应为 `full`。

### depends_on

- `depends_on` 指向的 change **归档后**仍可用原 `change_id`（目录进 `archive/YYYY-MM-DD-*`）；apply 前确认依赖已归档或本分支已落地其行为。

## delayed-changes（搁置提案 park）

`llmanspec/delayed-changes/` 收纳**未启动**的搁置提案（可按 `tui/`、`models/`、`tools/` 等分类；`legacy/` 存旧代提案）：

- park 内只放 proposal / design / tasks / research 等规划工件；**禁止** `specs/`（spec landing 只发生在 propose 之后）。
- 启动实现 = 经 propose / ff 正式化迁回 `changes/`；**归档时必须删除 delayed 副本**（一 id 一处真值）。
- 阶段性清淤时对照归档清点：已落地副本删除、已放弃提案删除、退役格式工件（如 `spec.toon`）不留。

## 指针

- 架构 SSOT（分层、不变量、seam、Xy\*、Provider 适配）：`src/AGENTS.md`。
- 高维产品/业务图：`docs/architecture/`（入口 `README.md`）。
- 命令与测试：根 `AGENTS.md`「命令」/「提交与测试」段。
- change 临时调研落点：上文「research（change 附属调研）」；耐久主题底稿才进 `docs/research/`。
