# language: zh-CN
# capability: test-standards
# purpose: 测试标准与约定 — 测试组织、harness 要求与覆盖期望。
# scope: tests/, src/

功能: test-standards

  @req:ts01 @human
  场景: BDD-vs-Unit 测试分界
    - 项目 MUST 维护 documented 边界：BDD 场景覆盖端到端编排（agent loop、tool execution、session lifecycle、CLI dispatch）；单元测试覆盖纯算法、数据结构契约、组件内部状态机与错误类型行为。测试 MUST NOT 在两层间重复覆盖。

  @req:ts02 @human
  场景: 核心数据类型测试覆盖
    - agent/ 会话词汇与 protocol/ 端口签名类型 MUST 有 #[cfg(test)] 模块验证关键路径：（1）serialization round-trip（适用时），（2）Display/From 转换，（3）constructor invariants。MUST NOT 再以独立 domain/ 或 runtime_protocol/ 顶栏作为测试挂载点。

  @req:ts03 @human
  场景: 纯逻辑组件测试覆盖
    - 以下 agent/ 模块 MUST 有 #[cfg(test)] 验证纯逻辑行为：queue.rs（MessageQueue 全部操作）、retry.rs（状态转换）、commands.rs（slash 解析）、config_value.rs（resolution）。MUST NOT 要求已移除的 prompt templates.rs 展开测试。

  @req:ts04 @human
  场景: Session 子组件测试覆盖
    - 以下 agent/ 子组件 MUST 有 #[cfg(test)]：model_manager.rs（cycle/select/thinking）、tool_manager.rs（register/filter）、skill_manager.rs（activation/XML 展开）。
