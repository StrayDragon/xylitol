---
title: sdk 收纳边界判定准则（wayfinder:explore）
depends_on: []
needs_specs_change: false
branch: sdd/c3355-refactor-sdk-inclusion-boundary
base_branch: main
base_sha: 7fa241a40d8de0d7fbc2f62b486c59fa70b81645
---

# wayfinder:explore — sdk 收纳边界判定（共享件可否入 sdk）

## Why

四个收敛点（T2/T3/T4/T5）都要先回答同一问题：一件方言无关的共享代码能否落 xylitol-plugin-sdk。src/AGENTS.md 写 sdk「只收词表薄留（消息 / chunk / usage / thinking 选项 / Compat 名 / tool_wire 形状）」「不得把 HTTP、假模型、观测写出放进 plugin-sdk」，但现状已比表述宽：

- crates/xylitol-plugin-sdk/src 已有 loop_middleware.rs 825 行、obs_gate.rs 259、token_estimator.rs 186、obs_session.rs 104、file_lock.rs 82；
- sdk Cargo.toml:16 已依赖 fastrace（ports/compaction.rs、token_estimator.rs、generate.rs 在用 SpanContext）；reqwest 不在。

边界无成文判定准则，四张归属票若各自解释会得出四套口径。

## 待决策问题

1. sdk 可收纳的判定准则：纯数据 / 纯函数可否；fastrace 语义（SpanContext 已入）与「观测写出」的界线；reqwest 等传输库依赖是否一票否决。
2. 准则之外的落点选项：新共享 crate（受 src/AGENTS.md crate 划界约束）／插件各自保留并显式接受镜像为决策，各自的适用条件。
3. 若准则与 src/AGENTS.md「词表薄留」表述有张力：修订表述还是收紧现状（改前须用户确认）。

## 事实底账（2026-11-17 审查实测）

- sdk 模块行数与 fastrace 依赖见上文 Why；reqwest 在 sdk 无引用。
- 既有共享先例：parse_tool_args、plan_message_history_travel（app-tui harness.rs:19 经 sdk 引入）、mcp.rs 254 行、codemode.rs。

## 完成判据

一页准则，给定任意一件共享代码可机械判定「可入 sdk / 需另找落点 / 接受镜像」；T2/T3/T4/T5 直接引用，不再各自解释。

## Open Questions（逐问深挖记录）

- **Q1 依赖政策（已定 2026-11-18）：分类准入。** 小而纯、无传递重依赖的解析/工具库可个案准入（partial-json-fixer 准入，dto/json_parse 迁 sdk）；带传输栈的库一票否决（reqwest 不入 sdk；provider/reqwest_headers.rs 49 行 ×2 留插件，显式记录「接受镜像」为决策）；attribution.rs 的 is_opencode_host 2 行 reqwest::Url 随件留插件（见 c3360 research 判定）。后续准入个案须在本节登记。
- **Q2 sdk→protocol 依赖边（已定 2026-11-18）：开单向边。** 翻译合约单副本须同时命名 Command（protocol）与 payload 类型（sdk），现无此 crate；protocol 依赖面轻（base64/serde/serde_json/strum）、边无环；排除 protocol 本体（信封变厚）/ runtime（明文协议无关）/ wire（kernel 不可达）/ 新 crate（明文禁止）。「不得认识信封」改述为不得承担信封职责（JSON-RPC envelope / 分派），不禁止引用协议数据类型；「框内三包互不依赖」改述为 tui 与 sdk / protocol 互不依赖 + sdk→protocol 单向边。c3385 走「翻译合约整体入 sdk」。
- **Q3 准则成文（已定 2026-11-18，用户确认措辞）：** 四条收纳准则写入 src/AGENTS.md「### sdk 收纳准则」；crate 表 sdk 行「不得认识信封」改述为「承担信封职责」、必须列补单向边；mermaid 子图标签改述并补 sdk→protocol 边。完成判据达成：任意共享件可按四条机械判定，c3375/c3380/c3385/c3390 直接引用本节不再各自解释。

## Further Notes

（research / 决策结论回写处）
