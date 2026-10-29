# language: zh-CN
# purpose: 第三方外进程插件最小切片：配置驱动的信任门禁、newline-delimited JSON-RPC 2.0 stdio 握手、工具桥与失败隔离。市场、签名、dylib 热载与同进程第三方 Rust 静态链入不在本文件范围。

功能: infra-plugin

  @req:r2990
  规则: 外部插件配置与信任
    仅 `external_plugins` 中 `trusted: true`、name 与 command 非空、name 不与核心/官方插件冲突且不重复的条目 须装配；未 trusted、保留名、重复名 须跳过并产生可观察 warn；无配置 须零装配。外部插件只经配置声明，不得为未配置插件付运行时成本。

    场景: 受信外部插件装配工具
      假如 配置含受信外部插件 fixture
      当 内核装配
      那么 工具注册表含 fixture__echo

    场景: 未受信外部插件不装配
      假如 配置含未受信外部插件 fixture
      当 内核装配
      那么 工具注册表不含 fixture__echo

    场景: 保留名外部插件不装配
      假如 配置含名为 tools-builtin 的受信外部插件
      当 内核装配
      那么 工具注册表不含 tools-builtin__echo

  @req:r2991
  规则: 外部插件协议与工具桥
    外部进程 须经 newline-delimited JSON-RPC 2.0 stdio 握手：initialize 须声明 `protocol_version: 1`、`tools` 能力与工具列表；工具公开名 须为 `{plugin}__{tool}` 且 provider-safe；`tools/call` 结果 须以文本内容回传。协议版本不匹配、缺少 tools 能力、进程退出 须失败可观察且不得静默成功。

    场景: 外部插件握手与调用
      假如 外部插件 fixture 声明 echo 工具
      当 初始化并调用 echo
      那么 返回文本 hello

    场景: 协议版本不匹配拒绝
      假如 外部插件 fixture 声明 protocol_version 99
      当 初始化外部插件
      那么 握手失败且错误含 protocol mismatch

  @req:r2992
  规则: 外部插件失败隔离与清理
    单个外部插件 spawn 或握手失败 须 warn 且不得导致整次内核装配失败；插件卸载 须移除其注册的工具并终止外部进程。工具注册 不得覆盖既有同名工具。

    场景: 失败插件不阻断装配
      假如 配置含命令不存在的受信外部插件
      当 内核装配
      那么 装配成功且无该插件工具
