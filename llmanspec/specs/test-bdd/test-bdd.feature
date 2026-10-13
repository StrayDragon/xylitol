# language: zh-CN
# capability: test-bdd
# purpose: BDD 测试框架：rstest-bdd — feature 文件、步骤定义与场景验证。
# scope: tests/bdd/, llmanspec/specs/
# reviewed-through: c2890

功能: test-bdd

  @req:r37
  规则: BDD 全量通过
    BDD 场景集 须保持全量通过（cargo test --test bdd）；未实现的孤儿 feature 须移除而非保留为债务，其行为由对应模块单测 / harness 覆盖。产品 TUI 交互护栏 须由 harness/单测与 live app-tui-* feature 共同覆盖。
    # verified-by: tests/bdd.rs

    场景: bdd-suite-fully-wired
      当 读取 BDD 套件接线
      那么 全量通过且无孤儿 feature
  @req:r1811
  规则: server-integration-scenarios
    BDD 测试套件 须含 server.feature（server 启动、health check、prompt 提交、event streaming、shutdown）与 approval.feature（经 reverse RPC 的工具审批往返）的步骤定义。
    # verified-by: tests/bdd/bindings_server.rs

    场景: server-integration-scenarios-bound
      当 读取 server 集成场景清单
      那么 含启动与健康与提交与流式
  @req:r1812
  规则: rstest-bdd-current
    仓库 须使用 crates.io 当前 rstest-bdd / rstest-bdd-macros 兼容最新 beta；既有核心 BDD 场景 须在升级后仍全部通过。
    # verified-by: Cargo.toml

    场景: rstest-bdd-current
      当 读取 BDD 依赖版本
      那么 使用 crates.io 当前版本
  @req:r1813
  规则: Partitioned live feature BDD
    BDD 场景 须支持在 live llmanspec/specs/<capability>/<capability>.feature 中以 @req 绑定 requirement，并由 tests/bdd/ 下的 #[scenario(path=..., name=<scenario.id>)] 消费；场景标题 须等于 scenario.id；该链路 须与现有 tests/features/ 手写链路并存，互不干扰。可执行 GWT 写在 .feature；架构/静态断言留在 spec.toon（feature:false）并由单测覆盖。不得依赖已移除的 solidify / feature.delta。
    # verified-by: tests/bdd/suite.rs

    场景: live-feature-partition-bound
      当 读取 BDD 绑定机制
      那么 经 @req 绑定且支持 live 分区
  @req:r1814
  规则: 配置 BDD 分层
    配置加载行为（三层合并、模型/provider 解析、ConfigValue 插值、默认值）须由 infra 层配置与 settings 的单元测试覆盖，不得保留无 step 实现的孤儿 feature 作为 BDD 债务。

    # verified-by: llmanspec/config.yaml

    场景: config-behavior-unit-covered
      当 读取配置行为测试分层
      那么 由 infra 单测覆盖

  @req:r1913
  规则: BDD 独立测试目标承载
    BDD 场景 须由独立集成测试目标承载（tests/bdd.rs，经 cargo test --test bdd 全量执行），不得以 src/tests.rs mod bdd 挂回 lib 测试配置（编译隔离：BDD 曾占 lib 测试编译 ≈53%，独立 target 后 lib 测试编译回落到 ≈20s）。
    # verified-by: tests/bdd.rs
    # verified-by: src/tests.rs

    场景: bdd-isolated-target-bound
      当 读取 BDD 挂载接线
      那么 独立测试目标承载且未挂 lib
