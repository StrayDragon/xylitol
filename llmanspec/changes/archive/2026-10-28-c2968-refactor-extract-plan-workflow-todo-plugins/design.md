# Design

## Crates

| Crate | Owns | Depends on |
|---|---|---|
| `xylitol-official-plugin-todo` | todo middleware + tools + `SessionAgentTodoGateway` | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-workflow` | workflow middleware + workflow tools | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-plan` | plan-mode middleware + plan tools + `SessionPlanDocGateway` | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-loop-example` | example middleware, self-heal, generator-critic, tool-exposure | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-tools` | core tools + `ask` | `xylitol-plugin-sdk` |

Rules:

- Plugins only `use xylitol_plugin_sdk`. No plugin-to-plugin crate dependency.
- Plan needs todo handoff: it resolves `AgentTodoGateway` from `Ctx` at apply time; if absent, `plan_submit` reports that todo handoff is unavailable.
- Plan needs approval: it resolves `AskUserGateway` from `Ctx`; if absent, `plan_submit` reports approval unavailable.
- Kernel supplies those services before plugin `apply`.

## Config

```yaml
todo:
  enabled: true
  progress: false
  reminder: false
  reminder_stale_after: 3
  checkpoint: false
  max_settles: 8
workflow:
  enabled: false
  max_replans: 3
plan:
  enabled: true
  read_only: true
```

`plan.enabled` stays true to preserve the `c2973` default-on decision; `workflow` remains opt-in.

- Old `loop_example.todo_progress` / `todo_reminder` / `todo_reminder_stale_after` / `todo_checkpoint` / `plan_mode` / `workflow` keys are rejected with a migration hint.
- `loop_example` keeps `enabled`, `max_settles`, `deny_bash_patterns`, `self_heal`, `generator_critic`, `tool_exposure`.

## Boot

1. Kernel boots core plugins and built-in tools.
2. Kernel registers services: `RuntimeSessionStore`, `AgentTodoGateway`, `AskUserGateway` (when available).
3. `todo` plugin applies (default-on), registering middleware and tools.
4. `workflow` / `plan` plugins apply only when enabled.
5. Each plugin returns an unregister effect so unload leaves no middleware or tools behind.

Tool names stay stable (`todo_rewrite`, `todo_update`, `workflow_*`, `plan_*`); only ownership changes.

## Specs

- `runtime-config`: replace `loop_example` sub-keys with `todo` / `workflow` / `plan` sections.
- `workspace-assembly`: new crate members and dependency direction.
- `agent-todo` / `agent-workflow` / `agent-plan`: ownership wording only; behavior unchanged.
- `src/AGENTS.md`: crate table, plugin boundary and default-on/off rules.

## Tests

- Plugin registration/unregistration per crate; disabled plugin registers nothing.
- Config schema/example drift test; old keys rejected.
- Tool ownership test: core tools only in `tools-builtin`; todo/workflow/plan tools only in their plugin.
- Service absence: plan plugin without ask/todo degrades with a structured error.
- BDD: default todo enabled, plan enabled, workflow disabled; registration scenarios moved to the owning plugins.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：`runtime-config` 改为 `todo` / `workflow` / `plan` 段；`workspace-assembly` 记录新 crate 与依赖方向；`agent-todo` / `agent-workflow` / `agent-plan` 改归属措辞；`src/AGENTS.md` 同步。测：`llman-sdd validate --all --strict`。
- 2. **todo 插件**（plugin）：新建 crate，迁移 `TodoProgressMiddleware` / `TodoReminderMiddleware` / `TodoCheckpointMiddleware`、`todo_tools`、`SessionAgentTodoGateway` 与配置。测：注册/卸载、工具归属。
- 3. **workflow 插件**（plugin）：新建 crate，迁移 `WorkflowMiddleware` 与 workflow 工具/配置。测：注册/卸载、工具归属。
- 4. **plan 插件**（plugin）：新建 crate，迁移 `PlanModeMiddleware` 与 plan 工具/配置；从 `Ctx` 解析 `AgentTodoGateway` / `AskUserGateway`，缺失时结构化降级。测：服务缺失降级。
- 5. **瘦身 loop-example / tools**（plugin）：`loop-example` 只留示例、自愈、评审、工具收窄；`tools-builtin` 只留核心工具与 `ask`。测：工具名闭集与旧注册移除。
- 6. **配置与接线**（config/kernel）：schema/example 新段；旧键拒绝并提示迁移；kernel 按配置装配与卸载。测：schema 漂移、旧键错误、默认 todo on / plan+workflow off。
- 7. **BDD**（tests）：注册场景迁到各自插件；默认开关断言。测：`cargo test --all-features --test bdd`。
- 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
