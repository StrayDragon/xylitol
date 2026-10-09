# language: zh-CN
# capability: test-infra
# purpose: 测试基础设施：Fake/Faux provider、临时目录 RAII、TUI E2E 隔离。
# scope: src/, tests/

功能: test-infra

  @req:r39
  规则: faux-provider
    System MUST 提供 FauxProvider，返回预配置的响应步骤，无需网络调用，用于确定性 agent 测试。
    # verified-by: src/infra/provider/fake.rs

    场景: faux-provider-no-network
      当 读取 faux provider 装配入口
      那么 按响应步骤返回且无需网络
  @req:r46
  规则: test-harness-tools
    System MUST 通过 rstest-bdd 场景/测试函数测试工具，使用 given/when/then 类型化占位符步骤，每个场景对应一个测试函数。
    # verified-by: tests/bdd/suite.rs

    场景: bdd-harness-step-typed
      当 读取 BDD 测试基建清单
      那么 场景以类型化占位符步骤且逐场景一测试
  @req:r54
  规则: temp-file-raii
    所有创建临时文件的测试 MUST 使用 RAII 清理（tempfile crate），执行后 MUST NOT 遗留产物。
    # verified-by: tests/bdd/helpers.rs

    场景: temp-file-raii-cleanup
      当 读取测试临时目录基建
      那么 RAII 清理且不留产物
  @req:r57
  规则: async-test-timeout
    异步集成测试 MUST 用 with_test_timeout 辅助函数包裹主体（默认 10s），防止 CI 因死锁或 mock 失败挂起。
    # verified-by: tests/bdd/helpers.rs

    场景: async-test-timeout-guard
      当 读取异步集成测试超时基建
      那么 包裹主体且防挂起
  @req:r60
  规则: no-fixed-tmp-paths
    测试 MUST NOT 使用固定名称的硬编码 /tmp 路径；MUST 使用唯一自动生成路径以支持并行执行。
    # verified-by: tests/bdd/helpers.rs

    场景: no-fixed-tmp-path
      当 扫描测试固定临时路径
      那么 使用唯一自动生成路径
  @req:r63
  规则: tui-e2e-isolation
    TUI 端到端测试（portable-pty 与 tmux）MUST 与主测试矩阵隔离：MUST 位于 workspace 级 tests/tui_e2e/（非 packages/xylitol-tui/tests/），MUST 标记 #[ignore] 或由 feature flag 门控，默认 cargo test 不运行，MUST 经专用 justfile 目标（test-tui-e2e）调用。tmux smoke MUST 额外要求 tmux 二进制存在（缺失则 skip 并给出清晰消息），并为 spawned session 设置 TERM=xterm-256color。每个 tmux session MUST 使用唯一名称（基于 PID 或时间戳）以支持并行，MUST 经 Drop guard 在 panic 时也 kill，避免泄漏 session。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: justfile

    场景: tui-e2e-workspace-isolation
      当 读取 TUI 端到端测试布局
      那么 独立于主矩阵
# re-review(c2835): 复审结论——本 capability 管辖行为不变；被测基建词表未动，仅测试判据与协议载体收口。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2837): flaky-fix 分支复核——测试时序放宽与诊断增强触及本 scope；行为不变。（2026-10-06）
