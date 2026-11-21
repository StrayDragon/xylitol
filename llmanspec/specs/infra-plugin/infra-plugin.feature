# language: zh-CN
# purpose: 第三方外进程插件最小切片：plugins/bin 发现与 describe 清单（含可选 mcp 声明）、安装后首用批准与批准持久化、config 显式停用、CLI 委派与命令捕获；工具统一经 MCP 管线（mcp__ 前缀）暴露，私有 stdio 工具桥已退役。市场、签名、dylib 热载与同进程第三方 Rust 静态链入不在本文件范围。

功能: infra-plugin

  @req:r2990
  规则: 外部插件发现与首用批准
    外部插件 须经 plugins/bin 目录发现与 describe 清单识别；发现不等于装配：首次工具表纳入前 须请求用户批准（对齐项目信任选择交互），批准状态 须持久化；未批准的插件 不得注册任何工具；config 须可随时显式停用外部插件，停用 须覆盖既有批准且优先级最高；无发现且无批准 须零装配，不得为未发现、未批准的插件付运行时成本。

  @req:r3377
  规则: 清单 MCP 声明注册
    外部插件清单 可选声明 mcp：声明 mcp 的插件 须自动注册为 MCP server，其工具统一经 mcp__ 管线暴露；未声明 mcp 的插件 不得注册工具。未批准插件的 mcp 声明 不得注册；批准后 须注册；config 停用 须覆盖批准。

    场景: 未批准插件不注册工具
      假如 发现清单声明 mcp 的外部插件且未经用户批准
      当 内核装配
      那么 工具注册表不含该插件的 mcp__ 工具

    场景: 批准后注册为 MCP 工具
      假如 发现清单声明 mcp 的外部插件且用户已批准
      当 内核装配
      那么 工具注册表含该插件的 mcp__ 工具

    场景: 停用覆盖批准
      假如 已批准的外部插件被 config 显式停用
      当 内核装配
      那么 工具注册表不含该插件的工具

    场景: 未声明mcp不注册服务器
      假如 发现清单未声明 mcp 的外部插件
      当 内核装配
      那么 该插件不注册为 MCP server

  @req:r3027
  规则: external-plugin-commands
    外部插件 必须 可声明 `commands` 能力与命令列表；内核 必须 把每个声明该能力的插件注册为 `plugin:<plugin-name>`；`plugin_command` 调用 必须 经 `commands/call` 透传参数并以文本回传结果；未注册命令、进程退出与超时 必须 返回可观察错误，不得 崩溃或静默成功。由内核单测覆盖。

  @req:r3074
  规则: 插件启停与配置节
    外进程插件配置 须收敛为 `plugins.enabled` 与 `plugins.disabled` 两个集合：enabled 显式纳入、disabled 显式停用且优先级最高；既有 `external_plugins` 配置节与 `trusted` 字段 不得 保留。外进程插件 仍可 在 initialize 中声明 `config_key` 与 `config_schema`；配置中对应顶层节 须 在插件装配后由插件按 rutis 依赖顺序校验；未声明 `config_key` 的插件 不得 认领任何顶层节；多个插件声明同一 `config_key` 须 可观察失败。由内核单测覆盖。

  @req:r3317
  规则: plugin-sdk-command-capture
    plugin-sdk 必须 提供 run_command_capture 助手：以 sh -c 执行命令、捕获 stdout、按字节上限截断、超时中止、消毒凭据环境变量；非零退出 必须 报错。由单测覆盖。
