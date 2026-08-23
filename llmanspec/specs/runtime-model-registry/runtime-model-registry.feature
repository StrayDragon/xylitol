# language: zh-CN
# managed by llman sdd partition-migrate
功能: runtime-model-registry

  @req:m10
  场景: reject-unsupported
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 max
    那么 失败且当前 level 不变

  @req:m15
  场景: reject-case-variant
    假如 当前模型支持集为 off 与 high
    当 set_thinking_level 为 HIGH
    那么 失败且当前 level 不变
