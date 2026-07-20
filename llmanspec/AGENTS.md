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
  - `priority` 为整数。**建议**用 5 的倍数（c05/c10/c1200）——目的是**预留加塞间隙**，方便后续插队；**非**强制 5 倍数。需要插队时直接占用相邻空位（c06/c07/c08/c09；实测 c996/c997/c998/c1156）。priority **MUST 唯一**。
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
  - `app-tui-*` — 产品 TUI（`src/app/tui`）；可多 capability，禁止把新产品合约堆进单体 `app-tui`（正经 c450 退役）
  - `agent-*` / `domain-*` / `runtime-*` / `infra-*` / `protocol-*` / `server-*` / `cli-*` — 对应分层
  - `test-*` — 测试基础设施（BDD harness、fake provider、qa-gate…）
  - `workspace-*` / `build-*` / `layer-*` / `user-*` / `architecture` — 仓库 meta / 跨层
  - 历史名可并存，迁移见并行清理任务。

### tasks

- 单 task 拆分上限约 2 小时。
- purpose-draft change 可只交付 `proposal.md`（`status: purpose-draft`）；apply 前须 promote 为完整工件（specs + tasks）。

## 语言

- spec 的 `purpose` / requirement `title`+`statement` / scenario `given`/`when`/`then` **MUST 中文**；技术标识符（类型名、路径、命令、req_id）保留英文。
- Gherkin `.feature`：BDD-on（Partitioned SSOT）下 `spec.toon` = 约束/不可执行场景；live `llmanspec/specs/<capability>/*.feature` = 可执行 GWT（`@req:`）。在非默认 feature 分支直接编辑二者 → `llman sdd change attach` / `checkpoint` → docs-only `change archive` → Git merge。**禁止** `solidify`、`change delta`、新建 `*.feature.delta.toon`。场景标题 MUST 用英文 `scenario.id`；可保留 rich Gherkin（Background / docstring / 并且）。与 `tests/features/` 手写链路可并存。

## BDD-on 操作闸（字段经验；上游正在收口）

Partitioned 双写与 checkpoint 时序已部分吸收进上游 llman（`improve-partitioned-ssot-agent-friction`）。
本段只保留 xylitol 仍要遵守的硬约束；CLI 缺口见 `../llman` change **`fix-sdd-bdd-on-change-stage`**。

### Partitioned 双写（MUST）

| 放哪 | 可执行场景（进 harness） | 仅文档场景 |
|---|---|---|
| `spec.toon` `scenarios[]` | **禁止**出现（无则 `scenarios[0]:`） | `feature: false` + GWT 可以 |
| `*.feature` + `@req:` | **唯一**可执行 GWT 正文 | n/a |

- **禁止**在 toon 写 `feature: true` 行（哪怕 GWT 与 `.feature`「看起来一样」——validate 报 `dual-write`）。
- 新需求：toon 只加 `requirements` 行；例子只加 `.feature` 场景。

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

### stage=draft（BDD-on）

已有 `proposal+design+tasks` 仍报 `draft` 时：通常是 **未 attach** → `llman sdd change attach <id>`（不要新建 `changes/<id>/specs/`）。attach 后应为 `full`。

### depends_on

- `depends_on` 指向的 change **归档后**仍可用原 `change_id`（目录进 `archive/YYYY-MM-DD-*`）；apply 前确认依赖已归档或本分支已落地其行为。

## 指针

- 架构 SSOT（分层、不变量、seam、Xy\*、Provider 适配）：`src/AGENTS.md`。
- 高维产品/业务图：`docs/architecture/`（入口 `README.md`）。
- 命令与测试：根 `AGENTS.md`「命令」/「提交与测试」段。
