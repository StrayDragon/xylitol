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

## 指针

- 架构 SSOT（分层、不变量、seam、Xy\*、Provider 适配）：`src/AGENTS.md`。
- 高维产品/业务图：`docs/architecture/`（入口 `README.md`）。
- 命令与测试：根 `AGENTS.md`「命令」/「提交与测试」段。
