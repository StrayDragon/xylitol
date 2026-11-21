# 调研底稿：xylitol-thoughtdag 耦合面与双库适配契约

探索于 /Users/l8ng/Projects/xylitol-thoughtdag（2026-11-20，只读）。

## 一、定位与规模

xylitol 的第一个外部 surface 插件：复用上游 ThoughtDAG 画布 UI，同进程桥接 xylitol 内核。Rust 侧 10 文件 1,969 行（xtd-core 7 模块 + xtd-web 单文件服务），web/app 为 SPA fork（约 150 个 TS 文件 3 万行）。近期提交均为裁剪性 SDD 变更，桥接已稳定（最后提交 2026-11-17）。

## 二、消费面

- 编译期 path 依赖随 xylitol 工作树浮动，无 rev 锁：xylitol（features 含 server）+ xylitol-plugin-sdk（xtd-core/Cargo.toml:8-9）。
- 运行时内嵌：boot(Profile::Embed) 后取 kernel.runtime.driver（XyInProcessDriver），不 spawn、不直接读会话文件（kernel.rs:135-150）；直接文件接触仅 token / pid。
- XyDriver 方法面：run / run_turn / switch_session / list_sessions / load_session_entries / fork_session / select_model / set_thinking_level / restore_model / available_models / current_model / thinking_level / session_id / cancel_token / begin_session_prepare / wait_session_prepare。
- 消息形状：TurnRequest::new(Vec<AgentPart>) with_context with_system_prompt；AgentPart::text / Image（chat_request.rs:192-204）。
- XyEvent 只消费 5 变体（TextDelta / ThinkingDelta / ToolExecutionStart / ToolExecutionEnd / Error），其余通配忽略。
- 会话 JSONL：derive_turns 只认 Message 且 role=user（turn.rs:16-33）；NDJSON 直通整个 SessionEntry（main.rs:425-447）；fork 用 user_message_id + ForkPosition::Before。
- 插件身份：plugins/bin 二进制发现 + describe 清单 + CLI 委派（xylitol thoughtdag ui start）；不用工具桥。

## 三、三轴冲击裁定

- 会话 actor 化：接口形状不变则不受影响；风险点为 driver 句柄 Send 与流语义（其以 Arc<Mutex> 跨线程持 driver）。
- rutis 插件扩展：无交集；唯一协调点为发现清单字段（describe 的 manifest.rs:8-31）。
- 数据结构演进：暴露最大但宽进——新 entry 类型被 derive_turns 跳过；NDJSON 直通自动透传新字段；唯一破坏类是既有消息加必填字段（TurnRequest 形状变化）。

## 四、双库适配契约（lockstep，无兼容层）

1. 每轴落地批次附带 thoughtdag 同批适配（path 浮动依赖，编译即验证）。
2. 数据演进 additive-only 自动免疫；必填字段类变更必须双库同批。
3. thoughtdag 从外部插件转嵌入端：describe / install.sh 随 c3430 退役。
4. 其 surface-{plugin,bridge,auth} specs 在其仓内同步改。
