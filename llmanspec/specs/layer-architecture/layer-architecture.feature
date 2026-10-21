# language: zh-CN
# purpose: 分层改写期间仍要守住的产品可观察后果（print 同进程、RPC 入口、单写者、嵌入不占端口）。代码组织真源是 AGENTS 与规划 §2，本文件不钉模块布局。
# scope-note: 不描述 crate / 层目录；源码扫描闸已撤。

功能: layer-architecture

  @req:r1520
  规则: 队列改向可解析
    线协议 JSON 必须能解析 steer、follow_up、clear_queue。
    场景: 协议携带共享类型
      当 解析队列命令 steer、follow_up、clear_queue 的线协议 JSON
      那么 分别得到 Steer、FollowUp 与 ClearQueue 变体

  @req:r1531
  规则: print 是 CLI 子模式
    print 必须作为 CLI 子命令解析，--session 必须生效。
    场景: print为CLI子模式
      当 xylitol print --session sid --trust --model m hi
      那么 解析成功且 --session 生效

  @req:r1532
  规则: server 产品入口
    产品入口必须是 POST /rpc 与 WS /rpc（同一方法表）。不得以 /api/v1 REST 为产品真源。
    场景: 服务端表面RPC入口
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 host.describe
      那么 应答为产品成功且 rpc_id 回显
      并且 不暴露 /api/v1 产品 REST

  @req:r1533
  规则: server 健康与 RPC
    启动 server 必须暴露 POST /rpc、WS /rpc 与 GET /healthz。
    场景: 运行时在线于基建
      当 启动 app::server 运行时
      那么 暴露 POST /rpc、WS /rpc 与 GET /healthz

  @req:r1534
  规则: agent 可无会话跑一轮
    未绑定 session 时，无工具 agent 必须能跑完一轮并出现 AgentEnd。
    场景: agent运行无会话绑定
      假如 未注册 should_stop_after_turn 的无工具 agent
      当 运行 AgentRuntime
      那么 正常出现 AgentEnd 且恰好一轮 TurnStart

  @req:r1521
  规则: 元数据事件不崩模型
    经桥缝注入模型选择与 thinking 档位时，模型不得 panic，条目与相位必须不变。
    场景: 投影于agent渲染于应用
      当 以桥缝注入元数据事件（模型选择与 thinking 档位）
      那么 模型不 panic 且条目与相位不变

  @req:r1519
  规则: Server 与 Print 同缝重载
    idle 提交 /reload 必须经共享缝完成。
    场景: 重载共享按服务端与print
      当 以主机泵在 idle 提交 "/reload"
      那么 重载经共享缝完成且尾插 Reload 步进汇总

  @req:r1518
  规则: MCP 随配置切换工具集
    先后用两组工具配置装配时，工具集必须随配置变化。
    场景: MCP配置经嵌入缝
      当 以两组不同工具集的 fixture 配置先后装配
      那么 工具集随配置变化

  @req:r1516
  规则: TUI 消费 session-tree
    idle 提交斜杠 session-tree 必须打开会话树，且不得当成 prompt 调用 run。
    场景: 分发被消费按TUI
      当 以主机泵在 idle 提交斜杠 session-tree
      那么 会话树打开且未作为 prompt 调用 run

  @req:r1528
  规则: thinking 按族分流
    medium 与 off 档注入各族请求体时，Responses / Completions / Anthropic 必须按 compat 分流，off 档必须省略字段。
    场景: 厂商diff收纳于桥缝
      当 以 medium 与 off 档解析请求 thinking 并注入各族请求体
      那么 Responses 用 reasoning.effort 且 Completions 随 compat 分流且 Anthropic 随 compat 分流且 off 档省略字段

  @req:r1511
  规则: reload 双端一次
    idle 提交 /reload 必须完成一次运行时重载。
    场景: 重载双结束效果
      当 以主机泵在 idle 提交 "/reload"
      那么 重载经共享缝完成且尾插 Reload 步进汇总
      那么 运行时重载调用恰一次

  @req:r1512
  规则: print 不占监听
    print 必须同进程完成一轮对话，且不得要求先占用监听地址。
    场景: 嵌入print无监听
      当 以同进程驱动完成一轮 print 对话
      那么 对话完成且未要求监听器

  @req:r1513
  规则: 会话是最小单元
    /session-resume 必须打开 Resume 面板并列举可恢复会话。
    场景: 会话作为最小单测
      当 以主机泵注入可恢复会话列表后提交 "/session-resume"
      那么 Resume 面板槽打开且经驱动列举可恢复会话

  @req:r1514
  规则: 一 session 一写者
    同会话第二路写入必须被稳定处理，不得靠协议写者令牌误伤同进程第二客户端。
    场景: 第二次写者拒绝带冲突
      假如 向 POST /rpc 发送 prompt 请求
      当 server 处理 prompt
      并且 另一客户端对同会话发送 steer
      那么 同进程第二客户端不得因协议写者令牌被拒

  @req:r1535
  规则: 进程内驱动无 Deref 包装
    进程内驱动必须自身实现端口，不得经 Deref 包装把调用绕过端口。由类型与组合根覆盖，不得单独扩 BDD step。
