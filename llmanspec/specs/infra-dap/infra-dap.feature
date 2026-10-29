# language: zh-CN
# purpose: DAP 工具插件：未配置零装配；配置后注册 launch / stack / scopes / variables / evaluate / continue / disconnect，经 dapz 压缩输出进入模型工具语境。daemon 复用、多会话池与全 DAP 命令覆盖不在本文件范围。

功能: infra-dap

  @req:r2996
  规则: 未配置零装配
    配置不含 `dap` 时 不得注册任何 `dap_` 前缀工具、不得 spawn 调试适配器、不得引入可感知运行时成本。

    场景: 未配置DAP零装配
      假如 配置不含 dap
      当 内核装配 LSP DAP
      那么 工具注册表不含 dap 前缀工具

  @req:r2997
  规则: 配置后注册 DAP 工具
    `dap` 配置存在时 须注册 `dap_launch` / `dap_stack` / `dap_scopes` / `dap_variables` / `dap_evaluate` / `dap_continue` / `dap_disconnect`；`backend` 与 `cwd` 须驱动 dapz AgentHandle；首次工具调用才启动适配器；`dap_disconnect` 须释放会话；卸载 须 disconnect 并 terminate。工具失败 须以 XyToolError 可观察，不得静默空成功。

    场景: 配置DAP注册工具
      假如 配置含 dap backend debugpy
      当 内核装配 LSP DAP
      那么 工具注册表含 dap_launch 与 dap_stack 与 dap_evaluate 与 dap_disconnect
