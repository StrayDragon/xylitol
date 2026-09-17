# language: zh-CN
# capability: test-bdd
# purpose: BDD 测试框架：rstest-bdd — feature 文件、步骤定义与场景验证。
# scope: tests/bdd/, tests/features/, llmanspec/specs/

功能: test-bdd

  @req:r37 @human
  场景: BDD 全量通过
    - BDD 场景集 MUST 保持全量通过（cargo test --test bdd）；未实现的孤儿 feature MUST 移除而非保留为债务，其行为由对应模块单测 / harness 覆盖。产品 TUI 交互护栏 MUST 由 harness/单测与 live app-tui-* feature 共同覆盖。

  @req:r1811 @human
  场景: server-integration-scenarios
    - BDD 测试套件 MUST 含 server.feature（server 启动、health check、prompt 提交、event streaming、shutdown）与 approval.feature（经 reverse RPC 的工具审批往返）的步骤定义。

  @req:r1812 @human
  场景: rstest-bdd-current
    - 仓库 MUST 使用 crates.io 当前 rstest-bdd / rstest-bdd-macros 兼容最新 beta；既有核心 BDD 场景 MUST 在升级后仍全部通过。

  @req:r1813 @human
  场景: Partitioned live feature BDD
    - BDD 场景 MUST 支持在 live llmanspec/specs/<capability>/<capability>.feature 中以 @req 绑定 requirement，并由 tests/bdd/ 下的 #[scenario(path=..., name=<scenario.id>)] 消费；场景标题 MUST 等于 scenario.id；该链路 MUST 与现有 tests/features/ 手写链路并存，互不干扰。可执行 GWT 写在 .feature；架构/静态断言留在 spec.toon（feature:false）并由单测覆盖。MUST NOT 依赖已移除的 solidify / feature.delta。

  @req:r1814 @human
  场景: 配置 BDD 分层
    - 配置加载行为（三层合并、模型/provider 解析、ConfigValue 插值、默认值）MUST 由 infra 层配置与 settings 的单元测试覆盖，MUST NOT 保留无 step 实现的孤儿 feature 作为 BDD 债务。
