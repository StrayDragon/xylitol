# language: zh-CN
# capability: user-experience
# purpose: 用户体验 — 鉴权引导消息、错误消息与启动诊断。
# scope: src/app/cli/

功能: user-experience

  @req:r1838
  规则: login-help
    System MUST 提供登录引导消息（或等价入口），返回引用 /login 与文档路径的引导。

    场景: login-guidance-references-docs
      那么 登录引导含 /login 与文档路径
  @req:r1839
  规则: no-models
    无可用模型时，System MUST 提供无模型提示消息（或等价入口）。

    场景: no-models-guidance
      那么 无可用模型提示衔接登录引导
  @req:r1840
  规则: no-model-selected
    未选择模型时，System MUST 提供未选模型提示消息（或等价入口）。

    场景: unset-model-display
      那么 未选模型展示占位而非厂商默认名
  @req:r1841
  规则: no-api-key
    System MUST 提供无 API key 提示消息（或等价入口），含 provider 名称。
    # （c2827 合并：runtime-model-registry r1756 鉴权指引并入本条。）

    场景: no-api-key-names-provider
      那么 无 api key 提示含 provider 名
# re-review(c2835): 复审结论——本 capability 管辖行为不变；改动限于 TUI/attach 家族的步骤实现与测试判据。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
