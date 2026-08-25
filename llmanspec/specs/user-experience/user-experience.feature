# language: zh-CN
# capability: user-experience
# purpose: 用户体验 — 鉴权引导消息、错误消息与启动诊断。
# scope: CLI 应用面（print 等）

功能: user-experience

  @req:ux1 @human
  场景: login-help
    - System MUST 提供登录引导消息（或等价入口），返回引用 /login 与文档路径的引导。

  @req:ux2 @human
  场景: no-models
    - 无可用模型时，System MUST 提供无模型提示消息（或等价入口）。

  @req:ux3 @human
  场景: no-model-selected
    - 未选择模型时，System MUST 提供未选模型提示消息（或等价入口）。

  @req:ux4 @human
  场景: no-api-key
    - System MUST 提供无 API key 提示消息（或等价入口），含 provider 名称。
