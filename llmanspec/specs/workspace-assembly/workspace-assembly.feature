# language: zh-CN
# purpose: 装配官方插件时，未配置的 MCP 与 hooks 不得连外进程。ReAct 循环留在执行权威，作为不可配置核心插件。跨端仍走信封 fory-v3。仅 plugin-sdk 与 kernel 可声明 rutis；仅 kernel 可声明 rutis-loader。
# scope-note: 不写会话实例化、hooks 改走事件总线、热路径耗时。叶子插件经装配适配层使用 rutis-bridge 不在本文件范围。不钉 crate 路径与类型名。未配置 MCP、信封宣告、无会话跑一轮 与既有 capability 同观察点，本文件从装配视角再锁。

功能: workspace-assembly

  @req:r2910
  规则: 仅装配适配层依赖 rutis
    执行权威、官方插件、产品端不得直接依赖 rutis。直接依赖 rutis 的 crate 必须是 plugin-sdk 或 kernel；直接依赖 rutis-loader 的 crate 必须是 kernel。
    场景: 扫描工作区仅装配适配层声明 rutis
      当 扫描工作区各包对 rutis 的 Cargo 依赖
      那么 仅装配适配层声明了 rutis

  @req:r2911
  规则: 未配置 MCP 时不注册工具
    配置里没有 mcp 服务器或列表为空时，装配不得向工具集注册 mcp 前缀工具。
    场景: 空 mcp 列表不含 mcp 前缀工具
      假如 产品以 Print 面启动
      当 查询装配后的 ToolSet
      那么 装配不含 mcp 前缀工具

  @req:r2912
  规则: 未配置 hooks 时不启动脚本进程
    hooks 配置为空时，分发工具调用不得启动 hook 进程，结果必须为允许。
    场景: 空 hooks 分发结果为允许
      当 以空 hooks 配置分发工具调用
      那么 分发器为空且结果为允许

  @req:r2913
  规则: 产品端与 Host 之间走信封
    产品端与 Host 之间必须走信封。host.describe 必须宣告 fory-v3。不得把这一通道换成 rutis-bridge。
    场景: host.describe 宣告 fory-v3
      当 服务端在空闲端口上启动
      那么 host.describe 的 result 携带 wire 格式集合且含 fory-v3

  @req:r2914
  规则: ReAct 循环留在执行权威
    ReAct 循环必须留在执行权威，并作为不可经 loader rows 替换的核心插件。不得做成可卸的官方插件。未绑定 session 时，无工具 agent 必须仍能跑完一轮并出现 AgentEnd。
    场景: 无会话无工具仍跑完一轮
      假如 未注册 should_stop_after_turn 的无工具 agent
      当 运行 AgentRuntime
      那么 正常出现 AgentEnd 且恰好一轮 TurnStart
