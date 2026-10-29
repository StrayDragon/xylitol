# language: zh-CN
# purpose: LSP 工具插件：未配置零装配；配置后注册 lsp_diagnostics / lsp_symbols / lsp_hover，经 lspz 压缩输出进入模型工具语境。全协议覆盖、补全 / 重构 / 格式化与 daemon 复用不在本文件范围。

功能: infra-lsp

  @req:r2993
  规则: 未配置零装配
    配置不含 `lsp` 时 不得注册任何 `lsp_` 前缀工具、不得 spawn LSP server、不得引入可感知运行时成本。

    场景: 未配置LSP零装配
      假如 配置不含 lsp
      当 内核装配 LSP DAP
      那么 工具注册表不含 lsp 前缀工具

  @req:r2994
  规则: 配置后注册 LSP 工具
    `lsp` 配置存在时 须注册 `lsp_diagnostics` / `lsp_symbols` / `lsp_hover`；`backend` 与 `language` 须驱动 lspz AgentHandle；首次工具调用才启动 LSP server；卸载 须 shutdown。工具失败 须以 XyToolError 可观察，不得静默空成功。

    场景: 配置LSP注册工具
      假如 配置含 lsp backend basedpyright
      当 内核装配 LSP DAP
      那么 工具注册表含 lsp_diagnostics 与 lsp_symbols 与 lsp_hover
