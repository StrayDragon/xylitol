# Design

## Evidence

- `crates/xylitol-kernel/src/driver/in_process/mod.rs:45-54`：`ToolFeatures::default()` → `todo: true`、`plan: true`、`workflow: false`。
- `crates/xylitol-kernel/src/driver/in_process/mod.rs:194-198`：`if features.plan { tools.extend(plan_tools(...)) }`。
- `plugins/xylitol-official-plugin-plan/src/tools.rs:956`：`plan_tools` 返回 5 个工具。
- `llmanspec/specs/agent-todo/agent-todo.feature:30` / `:87`：todo 工具须在默认工具表且首轮冻表前进入可见工具表。
- `llmanspec/specs/agent-runtime/agent-runtime.feature:276`：工具表在首次 generate 后冻结；回合中途不得热加。
- `llmanspec/specs/agent-runtime/agent-runtime.feature:553`：tool-exposure 中间件只改写模型可见 `tool_schemas`，不删除工具、不改权限。

## Constraint

工具表冻结是硬约束：`plan.enabled=false` 后在 `/plan` 开启，不能直接热加工具；要么在会话创建时按配置决定，要么在模式切换时重定稿（需要新语义）。因此“按模式动态注册”不是零成本选项。

## Options

### Option A：保持默认常驻，量化并记录成本（推荐先做）

- 测量默认 schema 的 token 占比；若 plan 5 工具占比很小（例如 < 2%），保持现状。
- 在 `configs/example.yaml` / 文档中说明默认工具面。
- 优点：零行为变更；plan 开箱可用。
- 缺点：简单任务仍携带 plan 工具。

### Option B：默认 `plan.enabled=false`，`/plan` 开启

- 优点：默认工具面最小。
- 缺点：需要定义“开启后如何进入工具表”的冻表/重定稿语义；`/plan` 在会话中途开启会触发重定稿，影响 provider KV cache；用户必须先 `/plan` 才能用计划模式。
- 需要改 `runtime-config` 默认值与 `agent-runtime` 冻表条款。

### Option C：tool-exposure 在 execute 模式隐藏 `plan_*`

- 由一个独立中间件（不是 plan 中间件；`r2955` 明确 plan 中间件不得改写工具表）按当前模式过滤模型可见 schema。
- 工具仍注册，权限终裁不变；只在 execute 模式隐藏，plan 模式显示。
- 优点：不触碰冻表；按请求生效；模型看不到无用工具。
- 缺点：需要新中间件或扩展现有 `loop-policies` tool_exposure；execute 模式下模型无法主动调用 `plan_mode` 进入 plan（需保留 `plan_mode` 工具可见或由 `/plan` 入口承担）。

### Option D：进入 plan mode 时注册 plan 工具

- 需要改冻表语义（模式切换触发重定稿）。
- 优点：最省 token。
- 缺点：改动最大；与 `mcp8` 冻表语义冲突；provider KV cache 失效。

## Recommendation

先做 **Option A 的测量**：

1. 统计默认工具表 schema 的 token（plan 5 个 / todo 2 个 / core 7 个）；
2. 若 plan 占比可忽略，保持 A；
3. 若占比显著，优先 **Option C**（只影响可见性，不动冻表），并保留 `plan_mode` 工具可见以便模型主动进入 plan；
4. 不建议 B/D，除非产品明确要求“默认最小工具面”。

## Test seam

- 单测：`compose_tools` 在 `plan=false` / `todo=false` 时的工具集合；
- 若选 C：`before_llm_request` 过滤后的 `tool_schemas` 名称集合（execute 模式不含 `plan_write` 等，plan 模式包含）；
- token 统计可作为一次性 `lab_` 脚本，不进 `just qa`。

## Risks

- 隐藏 `plan_*` 可能导致模型无法主动进入 plan；需保留入口（`/plan` 或 `plan_mode`）。
- 默认值变更会影响现有用户配置与文档；需要迁移提示。
- token 测量依赖模型 tokenizer，结果随 provider 变化；应记录测量口径。

## Open questions

1. 默认工具面的 token 成本是否值得优化？（先测量）
2. 若优化，选 C（可见性）还是 B（默认关闭）？
3. `plan_mode` 工具是否必须常驻，以便模型主动请求进入 plan？
