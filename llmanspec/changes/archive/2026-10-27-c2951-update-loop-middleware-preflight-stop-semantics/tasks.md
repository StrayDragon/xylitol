# Tasks: c2951

## 1. spec 条款

- [x] 1.1 在 `llmanspec/specs/agent-runtime/agent-runtime.feature` 的 `@req:r2922` 规则句末尾追加预演停止语义（已运行 / 不得执行 / 不得假设放行即执行），不改既有句子的语义。
- [x] 1.2 在 `r2922` 块内新增嵌套场景「前序停止后批内中间件已运行」。

## 2. 场景实现

- [x] 2.1 `tests/bdd/steps_loop_middleware.rs` 增加可计数 `before_tool` 中间件（共享 `Arc<AtomicUsize>`），首个中间件对首个调用返回 stop。
- [x] 2.2 实现该场景的 given/when/then 步骤：断言第二个中间件被调用过，且事件流中只出现首个调用的工具执行事件。
- [x] 2.3 `tests/bdd/bindings_loop_middleware.rs` 绑定新场景。

## 3. 文档

- [x] 3.1 `src/AGENTS.md`「Loop 中间件」段补一句：预演先于执行，前序 stop 后后续中间件已运行但工具不执行，细节指向 spec。

## 4. 验证

- [x] 4.1 `cargo test --all-features --test bdd` 绿（新场景通过）。
- [x] 4.2 `llman-sdd validate --all --strict` 绿。
- [x] 4.3 `just qa` 绿。
