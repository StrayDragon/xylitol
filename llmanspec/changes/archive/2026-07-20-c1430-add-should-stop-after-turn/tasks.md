# Tasks: c1430-add-should-stop-after-turn

## 0. 合约（本波完成；不 apply）

- [x] 0.1 `git checkout -b feat/c1430-add-should-stop-after-turn`
- [x] 0.2 proposal `status: full` + design + tasks；live 改 `agent-runtime` / `runtime-config`
- [x] 0.3 `llman sdd change attach c1430-…`；`validate --strict --no-check` 绿

## 1. ReAct 停闸缝（apply 时）

- [x] 1.1 在 `TurnEnd` 之后调用可选单槽 `should_stop_after_turn`；`true` → `AgentEnd` 并 return
- [x] 1.2 stop 路径 MUST NOT drain steer / follow-up（严格 pi）
- [x] 1.3 删除 loop 内 `max_iterations` 判断与 `ReActConfig.max_iterations`
- [x] 1.4 单测：钩子第 N turn 返回 true → 无后续模型轮；follow_up 入队但不被本 run 消费

## 2. 装配与配置迁移（apply 时）

- [x] 2.1 删除 `AgentBuilder` / `AgentCapabilities` / composition / bootstrap 的 `max_iterations`
- [x] 2.2 删除 `AppConfig` profile 字段、`config.schema.json`、示例 YAML
- [x] 2.3 钉未知/残留 `max_iterations` 加载行为（见 design）；单测覆盖
- [x] 2.4 清理 `XyError::MaxIterations` 若已无生产引用

## 3. BDD 与校验（apply 时）

- [x] 3.1 live feature 场景 `should-stop-skips-followup` / `should-stop-emits-agent-end` step 落地并绿
- [x] 3.2 `llman sdd validate c1430-… --strict`（含 BDD 或 `--no-check` 结构门 + 另跑 bdd）
- [x] 3.3 `just qa` 绿；`change finalize`（用户要求时再 commit）
