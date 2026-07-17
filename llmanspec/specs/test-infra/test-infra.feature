# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-infra

  @req:r39
  场景: happy
    假如 FauxProvider 已配置 text 与 tool call 响应
    当 agent 循环以 FauxProvider 运行
    那么 所有响应无网络调用返回且 call_count 被跟踪

  @req:r46
  场景: scenario
    假如 rstest-bdd 步骤定义已存在
    当 cargo test -- test_read_file
    那么 测试通过，使用类型化占位符步骤与场景绑定

  @req:r54
  场景: lsp-cleanup
    假如 LSP 测试创建临时 .rs 文件
    当 测试完成（成功或 panic）
    那么 临时文件自动删除

  @req:r57
  场景: timeout-helper
    假如 异步测试使用 with_test_timeout(10, future)
    当 future 超过 10s
    那么 测试 panic 并输出 timeout 消息而非无限挂起

  @req:r60
  场景: parallel-safe
    假如 两个 config loader 测试实例并行运行
    当 两者均使用唯一临时目录
    那么 无文件冲突或测试干扰

  @req:r62
  场景: compiles-with-rstest
    假如 项目有 rstest 与 rstest-bdd dev-dependencies
    当 cargo test bdd
    那么 所有场景以 cargo test 语法通过，无需自定义 runner

  @req:r63
  场景: e2e-not-in-default
    假如 默认 cargo test 运行
    当 tests/tui_e2e/ 用例存在但 #[ignore]
    那么 默认 cargo test 不执行 portable-pty 或 tmux 测试（仅经专用目标运行）

  @req:r63
  场景: tmux-session-cleaned-on-panic
    假如 tmux smoke 测试执行中 panic
    当 Drop guard 运行
    那么 tmux session 被 kill，无名为 xyl_e2e_* 的孤儿 session（可通过 tmux list-sessions 验证）
