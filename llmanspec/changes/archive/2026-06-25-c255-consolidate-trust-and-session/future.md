# Future: c255-consolidate-trust-and-session

候选待办池。每项归类为 `now` / `later` / `drop`。

## now — 建议转为后续 change

(无 — 本次 change 已自洽闭环,无必须立即跟进的项。)

## later — 保留,带触发信号

### L1. 交互式信任 prompt UI(t4 完整体验)
- **现状**:`resolve_project_trusted` 的 `on_prompt` 回调签名已就绪,但 CLI(print 模式)无交互 UI,接线点传 `|_| None`(fallback deny)。
- **触发信号**:实现交互式 REPL / TUI 模式时。
- **落地**:新 change `add-interactive-trust-prompt`,在 `interface/` 实现 `on_prompt`(列出 TrustOption、读用户选择),resolve 传 `has_ui=true`。受影响 capability:`security-policy`。

### L2. `/trust` 命令实际分发(t6 完整体验)
- **现状**:`AgentSession::save_trust_decision(&TrustManager, trusted)` 已就绪作为分发点,但无交互循环触发它。
- **触发信号**:L1 的交互式模式落地后。
- **落地**:在交互 loop 的 `PromptResult::Handled { command: "trust", .. }` 分支调用 `save_trust_decision` + 重新 resolve + 应用到运行时资源加载。

### L3. trust 运行时实时生效(t5 完整体验)
- **现状**:`/trust` 持久化后需重启才生效(资源加载在启动时门禁)。
- **触发信号**:L2 落地后,UX 需求驱动。
- **落地**:实现 `AgentSession::apply_trust_decision(trusted)` 热重载项目资源(重新 ResourceLoader + 重建 prompt)。注意:design 2.4 原提"移除 set_project_trusted 裸切换器",热重载方法应命名为 apply_trust_decision 以反映"接收解析值"而非"裸翻转"。

### L4. `ar02` 阈值回顾
- **现状**:修订后 ar02 用 ~100 行作上下限阈值,但 `session/mod.rs` 仍 980 行(as32 软目标 600 未达)。剩余 ~980 行是 facade 固有编排(模型委托/会话管理/accessor/lifecycle/sandbox)。
- **触发信号**:session/mod.rs 再次膨胀,或需进一步解耦 orchestration 方法。
- **落地**:评估是否将更多 `impl AgentSession` 方法 impl-split 到子模块(如 `model_ops.rs`、`session_ops.rs`、`lifecycle_ops.rs`),或调整 as32 的 600 行目标为"facade 编排合理体量"的定性标准。

### L5. 资源级细粒度信任
- **现状**:trust 是目录级(project CWD)二值决策。
- **触发信号**:用户需要"信任项目但排除某些 skill/extension"。
- **落地**:扩展 trust store 为 per-resource 决策(如 `{path: {settings: true, skills: false}}`)。受影响 capability:`security-policy`(t2 扩展)。

## drop — 已拒绝

### D1. 删除 `SettingsManager`
- **现状**:`SettingsManager` 全 crate 零调用点(本 change 发现),与 trust 同为死代码。但其特征化测试 `test_project_trust_untrusted_clears_project_settings` 仍在(不接入运行时)。
- **决策**:**drop**(不在本 change 范围)。是否删除应单独评估 —— 它可能是为未来 user-preference 管理预留的脚手架。若确认废弃,新 change `remove-dead-settings-manager`。

### D2. pi 文档引用清理(c240 已做)
- **决策**:c240 已清除 pi 引用。本 change 新增代码全部独立描述,无 pi 引用。无需跟进。
