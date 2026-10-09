---
depends_on: []
needs_specs_change: true
branch: sdd/2026-10-review-fixes
base_branch: main
base_sha: c10be4e9983564fac14b797ff9c6defaff15c4cf
---

# BDD steps 卫生治理：r57 名实对齐 + steps_c2827 拆分与时序去脆弱化

## Why

PR #8 评审暴露 test-bdd/test-infra 两族债务：

1. **r57 名实不符**：`test-infra` r57 要求异步集成测试以 `with_test_timeout` 包裹主体，但 helper 落地后**零真实调用**——唯一「使用」是 steps_c2827 的文本探针断言该字符串存在于 helpers.rs（自证循环）。
2. **steps_c2827.rs 4341 行职责失控**：~110 处 `la_load()` 源文件文本探针与真行为步骤混杂，横跨 TUI host、todo、image、process、MCP、分层、观测、测试基建 8+ 域。
3. **零散时序脆弱点**：W3 两函数体逐字复制；W5 冗余 800ms sleep；W6 5ms 壁钟硬断言（重负载 CI 可偶发超限）；W7 跨规则硬编码会话 id 耦合（c2842 步骤借用 s-c2841）。

## What Changes

1. **r57 修订**（`test-infra.feature`）：MUST 收窄到「依赖外部时序（网络/子进程/挂钟等待）的异步集成场景」；补「MUST NOT 以文本断言替代真实包裹」。
2. **steps_c2827 按域拆分**：文本探针集中至显式标注的 `steps_layer_probe.rs`；真行为步骤按域归位（image/process/todo/host 等）；`bindings_c2827.rs` 同步按 spec 域拆分。步骤注册文本零变化（场景↔步骤映射不变量）。
3. **时序去脆弱化**：W3 合并重复函数体；W5 删冗余 sleep；W6 5ms 硬断言改相对判据；W7 解耦硬编码会话 id。
4. **r57 真实化**：外部时序依赖场景（kill-tree / resources_watch / serve 启动 / remote resilience）的等待主体以 `with_test_timeout` 包裹。

## Capabilities

- `test-bdd`（tests/bdd 结构与场景映射不变量）
- `test-infra`（r57 异步超时合约）

## Impact / 风险

- 拆分是纯搬移 + `use` 路径调整，风险在「步骤注册文本意外漂移」——以 `check_bdd_steps` 门禁 + BDD 全量对拍兜底（映射不变量：零文本变化）。
- r57 修订触及既有规则块（锁定规则报告制 WARNING，预期内）。
- 时序项各自独立可回退。
