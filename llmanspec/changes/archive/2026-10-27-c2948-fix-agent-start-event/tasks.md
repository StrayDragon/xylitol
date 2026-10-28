# 任务

- [x] 1. **发射点**（`xylitol-runtime`）：`run_react_loop` 在 `agent.turn` span 与 `turn_obs_parent` 构造之后、`'outer: loop` 之前 `yield XyEvent::AgentStart { session_id, model }`；`model` 取启动时 `ModelManager::current_model().id`（与既有 `model_api` 同一把锁），未选模型取空串。覆盖 `start_root_stream` 与 `pending_root_stream` 两条入口（二者共用 `build_live_react_stream`）。
  - 测：runtime 单测断言首个事件为 `AgentStart` 且 `session_id` / `model` 与绑定的 session、当前模型一致。

- [x] 2. **BDD 落地**（`agent-runtime`）：新增 `@req` 规则「run 事件边界」+ 两个可执行场景（首/末事件、字段口径）；`tests/bdd` 绑定 + 步骤，复用既有 `agent_submit_root` seam。
  - 测：`cargo test --all-features --test bdd` 绿，既有场景不回归。

- [x] 3. **门禁**：`just qa` 全绿；`llman sdd validate c2948-fix-agent-start-event --strict` 绿。

- [x] 4. **文档**：`llmanspec/specs/agent-runtime/` 规则即文档真源；`src/AGENTS.md` 无需改（不改分层与 seam）。
