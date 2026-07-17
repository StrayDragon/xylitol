# language: zh-CN
# managed by llman sdd partition-migrate
功能: workspace-structure

  @req:r40
  场景: app-layer-contains-entries
    当 变更后检查源码树
    那么 src/app 含 cli/、rpc.rs、server/、tui/、gui.rs 与 core/ 子层（composition、driver）；print 位于 cli/ 下

  @req:r47
  场景: happy
    当 cargo check --all-features 运行
    那么 所有 feature 门控模块无错误编译

  @req:r51
  场景: app-composes-agent-and-infra
    当 新 app 组合根接线 Agent
    那么 其为唯一同时 import crate::agent 与 crate::infra 的 app 模块

  @req:r55
  场景: modes-use-build-agent
    当 CLI、RPC、Server 或 TUI 需要 Agent
    那么 它们调用 app::core::composition::build_agent 而非重复接线

  @req:r58
  场景: server-feature-gated
    当 cargo check --no-default-features 运行
    那么 server 与 tui 代码未编译且 axum/ratatui 依赖未拉取
