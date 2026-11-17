# Tasks

- [ ] t1 `runtime`:`poll_tool_gate` 改纯被动 `hub.view()` 轮询(删 `hub_wait`/窗口常量),门限耗尽一次性回落;新增计数 stub hub 单测(零 ensure/wait 调用、完成即冻结、门限回落一次通知)。验证:`cargo nextest run -p xylitol-runtime` 绿。
- [ ] t2 复验与门禁:arm freeze BDD 两场景绿;tmux 真机复现(准备期提交)tray 不抖、cue 常驻、Assembling 正常、冻结后自动上行;fmt / clippy / uncomment;`llman-sdd validate c3347-fix-tool-gate-passive-poll --strict` 全绿。
