# Tasks

- [x] t1 `runtime`:`AutomationEngine::tick` 将 `load_leaf_branch` 的 `NotFound` 视为空分支;新增单测(未持久化会话 tick = 空动作;有事实会话行为不变)。验证:`cargo nextest run -p xylitol-runtime` 绿。
- [x] t2 门禁与复验:fmt / uncomment / clippy;触及 crate 套件 + BDD 全量绿;`just qa` 全量门禁;`llman-sdd validate c3348-fix-automation-unpersisted-session --strict` 全绿。
