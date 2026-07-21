# language: zh-CN
# managed by llman sdd partition-migrate
功能: test-standards

  @req:ts01
  场景: boundary-defined
    假如 正在添加新测试
    当 PR 作者查阅 AGENTS.md 测试章节选择层
    那么 测试避免在 BDD 与单元测试层重复覆盖

  @req:ts02
  场景: domain-types-covered
    假如 agent/ 会话词汇与 protocol/ 端口源文件存在
    当 运行 cargo test
    那么 相关 agent 与 protocol 模块报告 serialize/Display/From 行为测试通过

  @req:ts03
  场景: pure-logic-covered
    假如 agent queue/retry/commands/templates 及相关纯逻辑模块存在
    当 运行 cargo test
    那么 这些模块报告纯逻辑行为测试通过

  @req:ts04
  场景: sub-components-covered
    假如 agent model/tool/skill manager 模块存在
    当 运行 cargo test
    那么 这些子组件报告其提取职责的测试通过
