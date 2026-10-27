# 任务

## 切片 1：after_tool 结构化载荷

- [x] 1. **SDK 形状**（`xylitol-plugin-sdk`）：`AfterToolCtx.result: String` → `parts: Vec<AgentPart>` + `details: Option<Value>`；导出 `AgentPart`/`Value` 已可用，无需新依赖。
  - 测：SDK 单测构造 ctx 并断言字段可读写。

- [x] 2. **runtime 接线**（`tool_exec`）：中间件在 hooks 之后、quiet 之前接收结构化 ctx；quiet 仅对单 Text 的 write/edit 生效；details 回退规则（quiet None → ctx.details）。
  - 测：BDD 场景「结构化分片保真」「改写详情生效」；既有 write/edit quiet 单测保持绿。

- [x] 3. **示例插件与调用点**：`loop-example` 不改行为；编译期修正所有 `AfterToolCtx` 构造点。

## 切片 2：before_tool 改路由（批次前预演）

- [x] 4. **预演结构**（`tool_exec`）：拆出 `prepare_call()`（hooks → AgentHooks → middleware → permission，产出 `PreparedCall`），`run_one` 只负责执行与结果处理。
  - 测：BDD 既有 deny/改参场景保持绿（预演不改执行语义、拒绝/故障仍回写 tool-error）。

- [x] 5. **批次规划改口径**（`react`）：按 `PreparedCall.final_name` + 最终工具 `execution_mode()` 重新 `classify` 后 `plan_windows`；MCP 前缀仍强制 Sequential。
  - 测：BDD 场景「改路由按新分类重排」（路由到串行工具后不与并行安全调用重叠）。

- [x] 6. **权限与事件口径**：permission 用 `(final_name, final_args)` 重算 target；`ToolExecutionStart` 带最终 name/args；找不到目标工具即 tool-error。
  - 测：BDD（路由后权限重裁拒绝、路由到不存在工具报错、Start 显示最终 name）。

## 切片 3：compact 决策载荷

- [x] 7. **SDK 形状**：`CompactRequest { instructions, reason }`；`DecideContinueCtx.compact: Option<CompactRequest>`。
  - 测：SDK 单测（默认 None、可填）。

- [x] 8. **compaction 端口带 reason**：`XyCompaction::compact` 增 `reason: &str`；orchestrator 用它填 `CompactionStart/End.reason`；`compaction_reason_kind` 识别 `loop_middleware:` 前缀；既有调用点传 `"manual"`。
  - 测：orchestrator 单测（reason 透传、kind 映射不把 loop_middleware 归 manual）。

- [x] 9. **runtime 接线**：`LoopControl::Compact` 读 `ctx.compact`，缺省 reason 补 `loop_middleware:<id>`；压缩后结算进事件流回归断言；失败消息带 id。
  - 测：BDD（compact 决策 → `CompactionStart.reason` 含中间件 id；压缩后出现 `ContextTokenSettlement`）。

## 收口

- [x] 10. **文档**：`src/AGENTS.md`「Loop 中间件」段补路由预演与 compact 口径。

- [x] 11. **门禁**：`just qa` 全绿；`llman sdd validate c2945-update-loop-middleware-payloads --strict` 绿。
