# Tasks — c669-add-app-tui-scrollback-async-tint

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c669-add-app-tui-scrollback-async-tint --no-interactive`（propose 闸；`--strict` 待 apply 勾完剩余 tasks）
- [x] 2. `runtime_protocol`：引入 `BashExecOpts`；`XyBashExecutor::execute(command, opts)` 替换旧签名（全仓改完，无 shim）
- [x] 3. `infra/bash_exec`：chunk_tx 真上行；Full 时发送侧合流；测 stream + cancel
- [x] 4. `BashExecHandler` / `Driver::execute_bash` 透传 `BashExecOpts`（产品可传 `chunk_tx`）
- [x] 5. bridge：`append_bash_output` + UTF-8 拼码点；Done/`finish` 终态对齐 `XyBashResult`
- [x] 6. host：拆除 bang 内层 `select!`；单环扇入 Tick/Input/Agent/BashChunk/BashDone；dirty+Tick 渲染
- [x] 7. input：`bash_active` 第二 `!`/`!!` 硬拒绝（提示 + 恢复 editor）；非 bang 仍 steer
- [x] 8. 修订 `design/bash-mode.md`（流式 pending + 硬拒绝互斥）
- [x] 9. Harness：多帧 pending→success；hanging Esc→`(cancelled)`；第二 bang 硬拒；sticky-Esc 回归
- [x] 10. `just fmt` + 相关 `cargo test` / clippy 绿
