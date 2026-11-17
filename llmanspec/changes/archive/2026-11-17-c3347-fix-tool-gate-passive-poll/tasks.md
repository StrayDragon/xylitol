# Tasks

- [x] t1 `runtime`:`poll_tool_gate` 改纯被动 `hub.view()` 轮询(删 `hub_wait`/窗口常量),门限耗尽一次性回落;计数 stub hub 单测(零 ensure/wait 调用、完成即冻结、门限回落一次通知)。验证:`cargo nextest run -p xylitol-runtime` 绿。
- [x] t2 `app-tui`:`loaded_resources_snapshot` 的 unary payload 经 `with_session` 注入 `session_id`(host 侧可选字段,零协议变更),使端侧刷新不再命中无 session 的「遍历 slot 取首个」兜底(历史残留 slot 会推未完成视图)。验证:`cargo nextest run -p xylitol-app-tui` 绿。
- [x] t3 诊断日志:`mcp hub ensure_for` / `update_snapshot`、serve `resources watch push`、端侧 `resources downlink apply` 各加 debug 级一行日志(本次抖动定位依赖,保留为诊断面)。
- [x] t4 复验与门禁:tmux 真机——空闲 50s tray 仅启动期一次合法迁移零抖动;准备期提交全流程(Assembling + Follow-up 条 + cue 常驻 → 冻结后自动上行 → 流式);arm freeze BDD 两场景、触及五 crate 套件(1581)、BDD 全量(1226)绿;fmt / uncomment / clippy 干净;`llman-sdd validate c3347-fix-tool-gate-passive-poll --strict` 全绿。
