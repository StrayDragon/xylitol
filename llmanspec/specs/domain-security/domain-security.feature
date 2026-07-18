# language: zh-CN
# live Partitioned SSOT feature (domain-security)
# executable permission subset (migrated from tests/features/sandbox.feature)
功能: domain-security
  @req:r61
  场景: network-domain-block
    假如 沙箱引擎已初始化
    当 检查网络域名 "evil.com"
    那么 结果应为拒绝且原因含 denied_domains

  @req:r65
  场景: deny-write
    假如 沙箱引擎已初始化
    当 检查写入路径 "/project/.env"
    那么 结果应为拒绝且原因含 write_denied

  @req:r65
  场景: allow-write
    假如 沙箱引擎已初始化
    当 检查写入路径 "/project/src/main.rs"
    那么 结果应为允许

  @req:r35
  场景: xy-tool-approval
    假如 需审批的工具被 SecurityToolWrapper 包装
    当 调用工具
    那么 审批检查在 XyTool::execute 前运行且拒绝时阻止

  @req:r44
  场景: happy
    假如 user 配置试图允许禁止 pattern
    当 合并配置
    那么 禁止 pattern 仍被阻止

  @req:r50
  场景: path-field-bypass
    假如 安全启用且 forbidden_patterns=['/etc/**']
    当 grep 以 path='/etc/passwd' 调用
    那么 SecurityEngine 返回 Blocked

  @req:r53
  场景: mcp-default-deny
    假如 安全启用且无 MCP 允许列表
    当 agent 调用 mcp:server:tool
    那么 SecurityEngine 返回 Blocked 并附理由

  @req:r56
  场景: default-enabled
    假如 全新安装无配置覆盖
    当 SecurityEngine 初始化
    那么 enabled 字段为 true

  @req:r59
  场景: hook-kill-on-timeout
    假如 hook 配置 timeout=100ms
    当 hook 脚本运行 sleep 999
    那么 子进程被杀且动作为 Block（默认 fail-closed）

  @req:r64
  场景: permission-config
    假如 config.yaml 含 security.permission.filesystem.write_denied=['.env']
    当 加载配置
    那么 PermissionConfig.write_denied 含 .env 且无 SandboxConfig 类型

  @req:r65
  场景: s11-renamed
    假如 阅读文件系统强制要求
    当 应用变更后
    那么 引用 XyPermission 而非 SandboxEngine

  @req:r66
  场景: s12-renamed
    假如 阅读网络强制要求
    当 应用变更后
    那么 引用 XyPermission 而非 SandboxEngine

  @req:s13
  场景: permission-trait
    假如 定位 permission 边界类型
    当 检查名称
    那么 为 runtime_protocol::XyPermission 返回 XyPermissionVerdict，后端含 AllowAllPermission 与 GlobPolicy

  @req:s14
  场景: default-deny-read
    假如 permission.filesystem.read_allowed=['/home/user/project']
    当 read 工具读取 /etc/passwd
    那么 permission engine 返回 access-denied

  @req:r71
  场景: no-agent-trust
    假如 应用变更后
    当 运行 rg 'agent::trust' src/ tests/（排除已移除模块）
    那么 零匹配且 src/agent/trust/ 目录不存在

  @req:r72
  场景: parent-inheritance
    假如 trust store 将 /home/user 设为 true
    当 查询 is_trusted('/home/user/projects/foo')
    那么 经父继承返回 true

  @req:r72
  场景: child-override
    假如 trust store 将 /home/user 设为 true 且 /home/user/evil 设为 false
    当 查询 is_trusted('/home/user/evil')
    那么 返回 false（最近祖先胜出）

  @req:r73
  场景: override-wins
    假如 trust store 将项目标为 untrusted
    当 以 trust_override=Some(true) 调用解析
    那么 结果为 trusted 且原因为 Override

  @req:r73
  场景: no-inputs-auto-trust
    假如 目录无 .xylitol/ 且无 .agents/skills/
    当 无覆盖调用解析
    那么 结果为 trusted 且原因为 NoTrustInputs

  @req:r73
  场景: fallback-deny-no-ui
    假如 目录有 trust 所需输入、无存储决策、默认策略 Ask 且 has_ui=false
    当 调用解析
    那么 结果非 trusted 且原因为 FallbackNoUi

  @req:r74
  场景: callback-invoked
    假如 解析在 has_ui=true 时到达 Ask 步骤
    当 用户回调选择 Trust 选项
    那么 决策为 trusted 且原因为 UserPrompt，选择持久化到 store

  @req:r75
  场景: untrusted-blocks-project-settings
    假如 项目有 .xylitol/settings.json 且项目解析为未 trusted
    当 SettingsManager 加载 settings
    那么 项目范围 settings 不合并到有效 settings

  @req:r76
  场景: command-persists
    假如 项目 CWD 中有活动会话
    当 用户经产品命令面运行信任命令
    那么 经应用缝持久化到 trust store，后续解析返回持久化值，且本会话不自动重载项目资源

  @req:s15
  场景: advisory-not-security
    假如 读者检查 XyPermission 模块文档
    当 阅读文档
    那么 声明门控为建议性且非安全边界，并指向 OS 或容器隔离以实现真实 containment
