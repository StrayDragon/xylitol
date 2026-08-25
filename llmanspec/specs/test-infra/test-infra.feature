# language: zh-CN
# capability: test-infra
# purpose: 测试基础设施：Fake/Faux provider、临时目录 RAII、TUI E2E 隔离。
# scope: 主 crate, workspace 测试

功能: test-infra

  @req:r39 @human
  场景: faux-provider
    - System MUST 提供 FauxProvider，返回预配置的响应步骤，无需网络调用，用于确定性 agent 测试。

  @req:r46 @human
  场景: test-harness-tools
    - System MUST 通过 rstest-bdd 场景/测试函数测试工具，使用 given/when/then 类型化占位符步骤，每个场景对应一个测试函数。

  @req:r54 @human
  场景: temp-file-raii
    - 所有创建临时文件的测试 MUST 使用 RAII 清理（tempfile crate），执行后 MUST NOT 遗留产物。

  @req:r57 @human
  场景: async-test-timeout
    - 异步集成测试 MUST 用 with_test_timeout 辅助函数包裹主体（默认 10s），防止 CI 因死锁或 mock 失败挂起。

  @req:r60 @human
  场景: no-fixed-tmp-paths
    - 测试 MUST NOT 使用固定名称的硬编码 /tmp 路径；MUST 使用唯一自动生成路径以支持并行执行。

  @req:r63 @human
  场景: tui-e2e-isolation
    - TUI 端到端测试（portable-pty 与 tmux）MUST 与主测试矩阵隔离：MUST 位于 workspace 级 tests/tui_e2e/（非 packages/xylitol-tui/tests/），MUST 标记 #[ignore] 或由 feature flag 门控，默认 cargo test 不运行，MUST 经专用 justfile 目标（test-tui-e2e）调用。tmux smoke MUST 额外要求 tmux 二进制存在（缺失则 skip 并给出清晰消息），并为 spawned session 设置 TERM=xterm-256color。每个 tmux session MUST 使用唯一名称（基于 PID 或时间戳）以支持并行，MUST 经 Drop guard 在 panic 时也 kill，避免泄漏 session。
