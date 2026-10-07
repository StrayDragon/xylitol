# language: zh-CN
# capability: test-standards
# purpose: 测试标准与约定 — 测试组织、harness 要求与覆盖期望。
# scope: tests/, src/

功能: test-standards

  @req:r1834
  规则: BDD-vs-Unit 测试分界
    项目 MUST 维护 documented 边界：BDD 场景覆盖端到端编排（agent loop、tool execution、session lifecycle、CLI dispatch）；单元测试覆盖纯算法、数据结构契约、组件内部状态机与错误类型行为。测试 MUST NOT 在两层间重复覆盖。
    # verified-by: src/AGENTS.md

    场景: bdd-unit-boundary-doc
      当 读取测试分界文档
      那么 明示端到端与纯逻辑边界
  @req:r1835
  规则: 核心数据类型测试覆盖
    agent/ 会话词汇与 protocol/ 端口签名类型 MUST 有 #[cfg(test)] 模块验证关键路径：（1）serialization round-trip（适用时），（2）Display/From 转换，（3）constructor invariants。MUST NOT 再以独立 domain/ 或 runtime_protocol/ 顶栏作为测试挂载点。
    # verified-by: src/protocol/message.rs

    场景: core-data-types-tested
      当 扫描核心类型测试覆盖
      那么 关键路径有单测验证
  @req:r1836
  规则: 纯逻辑组件测试覆盖
    以下 agent/ 模块 MUST 有 #[cfg(test)] 验证纯逻辑行为：queue.rs（MessageQueue 全部操作）、retry.rs（状态转换）、commands.rs（slash 解析）、config_value.rs（resolution）。MUST NOT 要求已移除的 prompt templates.rs 展开测试。
    # verified-by: src/agent/runtime/react/tests.rs

    场景: pure-logic-components-tested
      当 扫描纯逻辑组件测试
      那么 队列与重试等有单测
  @req:r1837
  规则: Session 子组件测试覆盖
    以下 agent/ 子组件 MUST 有 #[cfg(test)]：model_manager.rs（cycle/select/thinking）、tool_manager.rs（register/filter）、skill_manager.rs（activation/XML 展开）。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: src/agent/capabilities/mod.rs

    场景: session-subcomponents-tested
      当 扫描会话子组件测试
      那么 模型与工具管理器有单测
# re-review(c2835): 复审结论——本 capability 管辖行为不变；门禁矩阵与标准词表未动。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2837): flaky-fix 分支复核——测试时序放宽与诊断增强触及本 scope；行为不变。（2026-10-06）
