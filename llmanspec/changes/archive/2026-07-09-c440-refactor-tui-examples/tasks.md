# c440 Tasks — refactor tui examples

## 阶段 1：收敛 example surface

- [x] 1.1 删除 `demo` / `showcase` / `show_all`
- [x] 1.2 重写 `agent_demo` 为 fake coding-agent 主场景
- [x] 1.3 取消 `src/` 中临时 demo 模块，example 逻辑留在 `examples/`

## 阶段 2：验收 harness 对齐

- [x] 2.1 保留并复用 `render_result`
- [x] 2.2 新增 `agent_demo_test`
- [x] 2.3 E2E PTY / tmux 改为指向 `agent_demo`

## 阶段 3：文档与规则

- [x] 3.1 在 `packages/xylitol-tui/AGENTS.md` 写明支持矩阵
- [x] 3.2 在 `_HANDOFF.md` 写明 examples 收敛与支持矩阵

## 阶段 4：校验

- [x] 4.1 `cargo test -p xylitol-tui`
- [x] 4.2 `cargo test --test tui_e2e -- --ignored pty_agent_demo_submit_flow_survives_enter`
- [x] 4.3 `llman sdd validate c440-refactor-tui-examples --strict --no-interactive`
