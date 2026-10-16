# language: zh-CN
# purpose: 测试基础设施：Fake/Faux provider、临时目录 RAII、TUI E2E 隔离。

功能: test-infra

  @req:r39
  规则: faux-provider
    System 须提供 FauxProvider，返回预配置的响应步骤，无需网络调用，用于确定性 agent 测试。

    场景: faux-provider-no-network
      当 读取 faux provider 装配入口
      那么 按响应步骤返回且无需网络
  @req:r46
  规则: test-harness-tools
    System 须通过 rstest-bdd 场景/测试函数测试工具，使用 given/when/then 类型化占位符步骤，每个场景对应一个测试函数。

    场景: bdd-harness-step-typed
      当 读取 BDD 测试基建清单
      那么 场景以类型化占位符步骤且逐场景一测试
  @req:r54
  规则: temp-file-raii
    所有创建临时文件的测试 须使用 RAII 清理（tempfile crate），执行后 不得遗留产物。

    场景: temp-file-raii-cleanup
      当 读取测试临时目录基建
      那么 RAII 清理且不留产物
  @req:r57
  规则: async-test-timeout
    依赖外部时序（网络 / 子进程 / 挂钟等待）的异步集成场景 须用 with_test_timeout 辅助函数包裹等待主体（默认 10s），防止 CI 因死锁或 mock 失败挂起；纯内存异步场景 SHOULD 复用同一辅助统一收口。不得以「断言该辅助存在」的文本探针替代真实包裹（名实不符即债务）。

    场景: async-test-timeout-guard
      当 外部时序场景以 with_test_timeout 包裹等待主体
      那么 超时辅助被真实调用且不得仅以源码探针自证
  @req:r60
  规则: no-fixed-tmp-paths
    测试 不得使用固定名称的硬编码 /tmp 路径；须使用唯一自动生成路径以支持并行执行。

    场景: no-fixed-tmp-path
      当 扫描测试固定临时路径
      那么 使用唯一自动生成路径
  @req:r63
  规则: tui-e2e-isolation
    TUI 端到端测试（portable-pty 与 tmux）须与主测试矩阵隔离：须位于 workspace 级 tests/tui_e2e/（非 crates/xylitol-tui/tests/），须标记 #[ignore] 或由 feature flag 门控，默认 cargo test 不运行，须经专用 justfile 目标（test-tui-e2e）调用。tmux smoke 须额外要求 tmux 二进制存在（缺失则 skip 并给出清晰消息），并为 spawned session 设置 TERM=xterm-256color。每个 tmux session 须使用唯一名称（基于 PID 或时间戳）以支持并行，须经 Drop guard 在 panic 时也 kill，避免泄漏 session。

    场景: tui-e2e-workspace-isolation
      当 读取 TUI 端到端测试布局
      那么 独立于主矩阵
