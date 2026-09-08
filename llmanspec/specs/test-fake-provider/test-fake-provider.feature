# language: zh-CN
# capability: test-fake-provider
# purpose: Fake 与 mock provider：用于 agent 行为的确定性测试。
# scope: src/infra/provider/

功能: test-fake-provider

  @req:r38 @human
  场景: fake-provider
    - System MUST 提供 FakeProvider，经统一装配路径（AdapterXyModel）暴露为 XyModel，经 ScenarioStep 序列返回预配置响应。

  @req:r45 @human
  场景: scenario-orchestration
    - System MUST 支持编排多步场景：text -> tool_call -> tool_result -> text，含可配置 delay 与经 ScenarioStep 的错误注入。
