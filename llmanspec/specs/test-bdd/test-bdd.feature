# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-bdd

  @req:bt1
  场景: server-feature-steps
    假如 server.feature 场景有 Gherkin 步骤
    当 BDD runner 解析它们
    那么 所有 server.feature 与 approval.feature 场景运行并通过

  @req:bt2
  场景: upgrade-and-core-green
    假如 Cargo.toml 已升至目标 rstest-bdd 版本
    当 cargo test --test bdd -- --test-threads=1
    那么 既有核心 feature 均通过
