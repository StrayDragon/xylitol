# Tasks — 会话阶段名自动生成与持久化

- [ ] 1. 配置与归一化：新增 `session.stage_name` 配置类型 / 默认 / 校验 / schema；新增 `normalize_stage_name` 与 `fallback_stage_name`；单测覆盖 ANSI / 控制字符 / UTF-8 截断 / 空串。
- [ ] 2. Runtime 生成：新增 `StageNameConfig` 与 `AgentCapabilities::generate_session_name(force)`；读首条用户 + 首条助手正文；独立模型请求；超时 / 失败回退；写 session name；单测覆盖正常 / 超时 / 空 / 已有名 / force。
- [ ] 3. 驱动与协议：新增 `XyInProcessDriver::generate_session_name`、`Command::GenerateSessionName`、registry 行、kernel `exec_generate_session_name`；TUI remote / harness 接线；协议与驱动测试。
- [ ] 4. Bootstrap 配置接线：`ResolvedAssembly` 读取 `SessionConfig.stage_name`，注入 `AgentCapabilities`；配置单测。
- [ ] 5. Fork 继承：`fork_session` 成功后复制父 session name；单测。
- [ ] 6. Specs：更新 `agent-session` / `runtime-config` / `protocol-app`，新增场景并绑定 BDD。
- [ ] 7. 验证：`llman-sdd validate c3110-add-session-stage-name --strict`；`cargo test -p xylitol-runtime` / `-p xylitol-plugin-sdk` / `-p xylitol-protocol` / `-p xylitol-kernel`；相关 BDD。
