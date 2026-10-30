# language: zh-CN
# purpose: 只读子代理最小切片：配置驱动注册 subagent_readonly，另建隔离 runtime 与临时会话，只暴露只读工具；父级中止传递到子任务。可写子代理、多子任务总览、常驻工作者与成本预算不在本文件范围。

功能: agent-subagent

  @req:r2999
  规则: 未配置零装配
    配置不含 `subagent` 时 不得注册 `subagent_readonly`、不得创建子 runtime、不得引入可感知运行时成本。

    场景: 未配置子代理零装配
      假如 配置不含 subagent
      当 内核装配子代理
      那么 工具注册表不含 subagent_readonly

  @req:r3000
  规则: 配置后注册只读子代理
    `subagent` 配置存在时 须注册 `subagent_readonly`；工具 须另建隔离 runtime 与临时会话，不共享父 session；可见工具 须只含配置允许的只读工具（默认 read / grep / find / ls / ffgrep / fffind），不得含 write / edit / bash；父级取消 须 abort 子 runtime。子代理返回最后 assistant 文本；空文本 须报错。

    场景: 配置子代理注册只读工具
      假如 配置含 subagent tools
      当 内核装配子代理
      那么 工具注册表含 subagent_readonly
