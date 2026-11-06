# language: zh-CN
# capability: agent-codemode
# purpose: codemode（PTC）运行时：工具暴露、catalog、声明、嵌套 dispatch、Wasmtime 隔离与限额。
# scope: 只覆盖 run_code 工具与工具暴露层；不做 provider 原生 PTC、mode only、多语言、嵌套事件树。

功能: agent-codemode

  @req:r3250
  规则: exposure-and-catalog
    XyTool 须可声明 exposure（direct/model-only/codemode/deferred/hidden），默认 direct；模型工具表 须只含 direct 与 model-only；codemode catalog 须含 direct/codemode/deferred，不得含 model-only/hidden。catalog 须按会话工具集、能力开关、trust 过滤，并复用 ToolSet freeze 快照。

    场景: exposure 决定模型与 codemode 可见面
      假如 工具集含 direct/model-only/codemode/deferred/hidden 工具
      当 生成模型工具表与 codemode catalog
      那么 模型只含 direct 与 model-only 且 catalog 含 direct/codemode/deferred

  @req:r3251
  规则: tool-exposure-intersection
    codemode catalog 与 dispatch 须与 loop_policies.tool_exposure 的有效模型可见集取交集；allow/deny/read_only 拒绝的工具 不得出现在 catalog，也不得被 dispatch 调用。

    场景: tool-exposure 拒绝的工具不进入 catalog
      假如 loop_policies.tool_exposure 拒绝 write
      当 生成 codemode catalog
      那么 catalog 不含 write

  @req:r3252
  规则: declaration-budget
    声明 须按 catalog 生成；deferred 工具 不得内联；inline 声明 须受 inline_budget 约束；超预算时 须退回 ALL_TOOLS/searchTools/describeTool 发现。

    场景: deferred 工具不内联
      假如 catalog 含 deferred 工具
      当 生成声明
      那么 声明不含该工具且 ALL_TOOLS 可发现

  @req:r3253
  规则: nested-dispatch
    嵌套调用 须复用 before_tool / after_tool / 权限 / hooks；不得递归 run_code；不得把 model-only/hidden 暴露给脚本；只有外层 run_code 结果 须进入模型上下文与 session。

    场景: 嵌套调用复用权限且拒绝递归
      假如 run_code 脚本调用 write 与 run_code
      当 执行脚本
      那么 write 经 before_tool 与权限且 run_code 被拒绝

  @req:r3254
  规则: wasmtime-limits
    run_code 须在 Wasmtime 隔离中执行；guest 不得访问 FS/网络；fuel、epoch 取消/超时、内存上限 须映射为可分类错误；输出 须受 max_result_bytes 限制。

    场景: fuel 耗尽与取消映射为错误
      假如 run_code 执行无限循环或收到取消
      当 执行脚本
      那么 返回 fuel_exhausted 或 cancelled 错误

  @req:r3255
  规则: zero-assembly-and-cache
    codemode 未配置或关闭时 不得注册 run_code、声明中间件或 catalog 服务；模块缓存字节 须可信并按版本/target/guest hash 校验；缓存失败 须回退编译。

    场景: 未配置 codemode 零装配
      假如 配置没有 codemode 节
      当 装配工具与中间件
      那么 不含 run_code 且无 codemode 中间件
