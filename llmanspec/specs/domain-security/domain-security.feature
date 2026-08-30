# language: zh-CN
# capability: domain-security
# purpose: 安全策略 — 含工具级门控的 permission 系统、trust store 与审批工作流。
# scope: src/infra/permission/, src/infra/trust/, src/protocol/

功能: domain-security

  @req:r35 @human
  场景: Security 包装器
    - SecurityToolWrapper MUST 包装 XyTool（非 adk_core::Tool），并在工具执行前强制审批策略。

  @req:r44 @human
  场景: 仅收紧
    - System MUST 仅允许三层配置覆盖收紧规则，不得放宽。

  @req:r50 @human
  场景: 统一 path 字段检查
    - SecurityEngine MUST 在评估文件系统工具访问时同时检查 'file_path' 与 'path' 参数字段。

  @req:r53 @human
  场景: MCP 工具安全策略
    - SecurityEngine MUST 对 MCP 工具（名称前缀 'mcp__'（及过渡期遗留 'mcp-' / 'mcp_' / 'mcp:'））应用专用策略分支，含显式 server/tool 允许列表与默认拒绝语义。

  @req:r56 @human
  场景: 安全默认启用
    - 生产构建中 SecurityConfig.enabled MUST 默认为 true。

  @req:r59 @human
  场景: hook 超时杀进程
    - Hook 执行器 MUST 在超时时杀死子进程，并 MUST 支持可配置 fail-open 或 fail-closed 行为。

  @req:r61 @human
  场景: 网络域强制
    - SecurityEngine MUST 对 bash/curl 与 MCP SSE URL 强制 network.allowed_domains 与 network.blocked_domains。该行为 MUST 有可执行 BDD 场景（live domain-security.feature / network-domain-block）。

  @req:r64 @human
  场景: Permission 配置结构
    - System MUST 定义 PermissionConfig 结构，含 enabled、backend（PermissionBackend，当前仅 Glob）、filesystem（read_allowed、write_allowed、write_denied）、network（allowed_domains、denied_domains）、process（allowed_paths），可从 config.yaml security.permission 键加载。

  @req:r65 @human
  场景: 文件系统权限检查
    - 文件系统工具（read、write、edit）SHALL 在执行前检查 XyPermission。路径匹配 write_denied 或（read_allowed 非空时）不匹配 read_allowed 时，工具 SHALL 返回 access-denied 错误。

  @req:r66 @human
  场景: 网络权限检查
    - Bash 执行 SHALL 对命令文本中出现的 URL 检查 XyPermission 的 network.allowed_domains 与 network.denied_domains。域被拒绝或（allowed_domains 非空时）不在允许列表时，执行 SHALL 被阻止。

  @req:s13 @human
  场景: Permission trait
    - System MUST 提供 protocol::XyPermission trait（自 XySandboxEngine 重命名），含 check_read(path)、check_write(path)、check_network(domain)、check_process(path)，返回 XyPermissionVerdict（自 XySandboxVerdict 重命名）。该 trait 为 ReAct 循环在工具分发前消费的进程内建议性 permission 门控，非安全边界。具体后端（禁用时 AllowAllPermission、配置模式匹配 GlobPolicy）位于 infra/。

  @req:s14 @human
  场景: Permission 默认拒绝未列出
    - permission.filesystem.read_allowed 非空时，任何不匹配允许 pattern 的路径 SHALL 视为拒绝（默认拒绝）。permission.network.allowed_domains 非空时，任何不匹配允许 pattern 的域 SHALL 视为拒绝。

  @req:r71 @human
  场景: 信任单一事实来源
    - 项目 trust 状态 MUST 在 infra 层 trust 模块有单一真源；agent 层 MUST NOT 含自有 trust store 或 trust resolver。

  @req:r72 @human
  场景: 信任持久化格式
    - Trust 决策 MUST 持久化为 JSON 文件，映射规范绝对目录路径到布尔 trust 决策（true=trusted、false=untrusted）或 null（已清除）。store MUST 支持父目录继承：祖先目录上的 trust 决策适用于全部后代项目目录，除非被覆盖。

  @req:r73 @human
  场景: 信任解析管线
    - 项目 trust 解析 MUST 遵循固定优先级：(1) 显式 CLI 覆盖（--trust / --no-trust）；(2) 无 trust 所需输入时自动 trust；(3) 带父继承的持久 store 查找；(4) 配置默认策略（always / never / ask）；(5) 仅 has_ui 为 true 时 app UI 提示；(6) 回退 deny。解析 MUST 同时返回决策与原因。

  @req:r74 @human
  场景: 信任 UI 回调注入
    - trust resolver MUST 接受 app 步骤的 on_prompt 回调，MUST NOT 直接做终端 I/O。app/ 层负责提供提示回调；agent/ 与 infra/ 层对 trust 决策 MUST 保持 UI 无关。

  @req:r75 @human
  场景: 信任状态接线
    - agent 启动时，项目 CWD 的已解析 trust 决策 MUST 应用于门控项目范围 settings 与资源。项目未 trusted 时 SettingsManager MUST 拒绝加载项目范围 settings。SettingsManager 上独立 project_trusted bool 切换 MUST 由已解析 trust 决策替换（per t1 单一真源）。

  @req:r76 @human
  场景: 信任命令接线
    - 信任相关产品命令（如 /trust）MUST 经 Driver/composition（或等价应用缝）调用 TrustManager/端口持久化决策；MUST NOT 从应用面 reach-in infra::trust 具体类型。会话中运行信任命令 MUST 为当前项目 CWD 持久化决策；写盘成功后本会话 MUST NOT 自动重载项目 skills/MCP/context（用户显式 /reload 或重启除外）。

  @req:s15 @human
  场景: Permission 非安全边界
    - 文档与命名 MUST 明示 XyPermission 为建议性：礼貌阻止循环调用被拒绝工具，但不阻止主机级访问，因 bash 仍可删文件且恶意 prompt 不受 containment。真实隔离 MUST 来自 OS、容器或 VM 边界（如未来将工具执行委托到沙箱的工具路由模式）；扩展 XyPermission MUST NOT 被视为增加安全控制。
  @executable @req:r61
  场景: network-domain-block
    假如 沙箱引擎已初始化
    当 检查网络域名 "evil.com"
    那么 结果应为拒绝且原因含 denied_domains

  @executable @req:r65
  场景: deny-write
    假如 沙箱引擎已初始化
    当 检查写入路径 "/project/.env"
    那么 结果应为拒绝且原因含 write_denied

  @executable @req:r65
  场景: allow-write
    假如 沙箱引擎已初始化
    当 检查写入路径 "/project/src/main.rs"
    那么 结果应为允许

  @executable @req:r35
  场景: xy-tool-approval
    假如 需审批的工具被 SecurityToolWrapper 包装
    当 调用工具
    那么 审批检查在 XyTool::execute 前运行且拒绝时阻止

  @executable @req:r44
  场景: forbidden-pattern-blocks-override
    假如 user 配置试图允许禁止 pattern
    当 合并配置
    那么 禁止 pattern 仍被阻止

  @executable @req:r50
  场景: path-field-bypass
    假如 安全启用且 forbidden_patterns=['/etc/**']
    当 grep 以 path='/etc/passwd' 调用
    那么 SecurityEngine 返回 Blocked

  @executable @req:r53
  场景: mcp-default-deny
    假如 安全启用且无 MCP 允许列表
    当 agent 调用 mcp_server_tool
    那么 SecurityEngine 返回 Blocked 并附理由

  @executable @req:r56
  场景: default-enabled
    假如 全新安装无配置覆盖
    当 SecurityEngine 初始化
    那么 enabled 字段为 true

  @executable @req:r59
  场景: hook-kill-on-timeout
    假如 hook 配置 timeout=100ms
    当 hook 脚本运行 sleep 999
    那么 子进程被杀且动作为 Block（默认 fail-closed）

  @executable @req:r64
  场景: permission-config
    假如 config.yaml 含 security.permission.filesystem.write_denied=['.env']
    当 加载安全配置
    那么 write_denied 字段含 .env 且配置键来自 security.permission 非 security.sandbox

  @executable @req:s13
  场景: permission-trait
    假如 permission 后端实例已构造
    当 调用 check_read("/tmp/test")
    那么 返回 XyPermissionVerdict 且默认后端为 AllowAllPermission

  @executable @req:s14
  场景: default-deny-read
    假如 permission.filesystem.read_allowed=['/home/user/project']
    当 read 工具读取 /etc/passwd
    那么 permission engine 返回 access-denied

  @executable @req:r72
  场景: parent-inheritance
    假如 trust store 将 /home/user 设为 true
    当 查询 is_trusted('/home/user/projects/foo')
    那么 经父继承返回 true

  @executable @req:r72
  场景: child-override
    假如 trust store 将 /home/user 设为 true 且 /home/user/evil 设为 false
    当 查询 is_trusted('/home/user/evil')
    那么 返回 false（最近祖先胜出）

  @executable @req:r73
  场景: override-wins
    假如 trust store 将项目标为 untrusted
    当 以 trust_override=Some(true) 调用解析
    那么 结果为 trusted 且原因为 Override

  @executable @req:r73
  场景: no-inputs-auto-trust
    假如 目录无 .xylitol/ 且无 .agents/skills/
    当 无覆盖调用解析
    那么 结果为 trusted 且原因为 NoTrustInputs

  @executable @req:r73
  场景: fallback-deny-no-ui
    假如 目录有 trust 所需输入、无存储决策、默认策略 Ask 且 has_ui=false
    当 调用解析
    那么 结果非 trusted 且原因为 FallbackNoUi

  @executable @req:r74
  场景: callback-invoked
    假如 解析在 has_ui=true 时到达 Ask 步骤
    当 用户回调选择 Trust 选项
    那么 决策为 trusted 且原因为 UserPrompt，选择持久化到 store

  @executable @req:r75
  场景: untrusted-blocks-project-settings
    假如 项目有 .xylitol/settings.json 且项目解析为未 trusted
    当 SettingsManager 加载 settings
    那么 项目范围 settings 不合并到有效 settings

  @executable @req:r76
  场景: command-persists
    假如 项目 CWD 中有活动会话
    当 用户经产品命令面运行信任命令
    那么 经应用缝持久化到 trust store，后续解析返回持久化值，且本会话不自动重载项目资源
