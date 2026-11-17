---
title: 重复收敛 wayfinder 地图：四个重复点的归属决策路径
depends_on: [c3355-refactor-sdk-inclusion-boundary, c3360-refactor-provider-scaffold-deps, c3365-refactor-truncation-spec-contracts, c3370-refactor-dispatch-twins-shape, c3375-refactor-provider-scaffold-home, c3380-refactor-truncation-policy-module, c3385-refactor-dispatch-contract-home, c3390-refactor-agent-lifecycle-component]
needs_specs_change: false
---

# 重复收敛 wayfinder 地图（provider 脚手架 / lsp·dap 生命周期 / 截断策略 / Command→Outcome）

## Why

四个重复收敛点（provider 脚手架实现重复 839 行/份、lsp/dap 生命周期双实现已漂移、截断策略 4 处实现、Command→Outcome 三写 + 跨 crate 双胞胎）各自需要「落点 + 公共形状 + 合约影响 + 实施路径」决策；事实面横跨 6 个 crate 群、决策面互相依赖（共享同一 sdk 收纳边界根决策），单会话装不下。本图把路径画成 change 依赖图，逐票解决决策。

## What Changes

- 决策工件：四张归属票各产出完结决策；三张查证票 research 已归档（c3360/c3365/c3370）。
- 架构文档：sdk 收纳准则四条与 sdk→protocol 单向边已随 c3355 落 src/AGENTS.md（已归档）。
- 后续实现：全部归属票关闭后按「实施分组」决策收拢为正式 llman-sdd-propose 计划；本图不直接交付代码。

## Destination（目的地）

四个重复收敛点各有完结决策——每项定落点、公共形状、合约影响、实施路径（propose / quick）——决策无遗留，可分别转 llman-sdd-propose / llman-sdd-quick 实施。

## Notes（备注）

- 本图源自 2026-11-17 架构审查（llman-sdd-arch-review）Worth exploring W1–W4；证据以该轮实测为准（路径:行号已录入各票「事实底账」）。
- 每会话推进前必读 skill：llman-sdd-wayfinder（推进规则）、xylitol-dev；逐问深挖用 llman-sdd-explore 深挖分支。
- 常设偏好：决策票结论必须引用可定位证据（路径:行号或命令输出）；sdk 边界判定以 src/AGENTS.md「三层契约与 Xy*」「Provider 与消息」两节为基准；改 src/AGENTS.md 表述前须取得用户确认。
- ticket 关闭方式：答案写入本票 proposal（Further Notes 摘要 research 结论），经 change start/attach 绑分支后 finalize；关闭后在本图 Decisions so far 追加一行要点。
- 查资料票产出落各自 changes/&lt;id&gt;/research/，关闭记录见 Decisions so far。

## Decisions so far（已定的决策）

- c3360 provider 脚手架依赖面盘点（已归档 archive/2026-11-17-c3360）：真实实现重复 839 行/份；obs_session 为 facade 非重复；trace / wire_policy / attribution（例外 2 行 reqwest::Url）/ thinking 核心可迁 sdk；reqwest_headers 与 json_parse 阻塞在 reqwest / partial-json-fixer 依赖，待 c3355 准则仲裁。
- c3370 dispatch 双胞胎对比（已归档 archive/2026-11-17-c3370）：DispatchOutcome 25 变体与三函数逐字相同；transport-only 名单实为三份逐字复制；推荐落点 plugin-sdk（sdk→protocol 依赖边 + 5 个数据类型下沉 + kind 化错误构造 + 翻译合约本体入 sdk，kernel 留 outcome_to_value），待 c3355 / c3385 采认。
- c3365 截断 spec 合约全集（已归档 archive/2026-11-17-c3365）：bash 脚注被 r1431 + BDD 步断言 + TUI 前缀识别三层逐字钉死、mcp 仅语义钉死 kept/dropped/limit 三要素——统一文案即合约变更，可行组合式（保留 bash 锚行 + 追加通用 bytes 行）或走 propose 改 r1431/r1345；必须保留的差异面与可统一面清单已备；r1140 max_lines 与实现的漂移移交 c3380 第 6 问。
- c3355 sdk 收纳边界（已归档 archive/2026-11-18-c3355）：四条收纳准则成文 src/AGENTS.md「### sdk 收纳准则」——①纯数据/纯函数无新依赖可入 ②fastrace 埋点可入、观测写出不得入 ③传输栈一票否决（reqwest 相关件接受镜像）、小纯解析库个案准入（partial-json-fixer 准入）④不造第 4 契约 crate；开 sdk→protocol 单向边（Command 词表），crate 表「不得认识信封」改述为「承担信封职责」。四张归属票 c3375/c3380/c3385/c3390 全部解锁。
- c3375 provider 脚手架归属（已归档 archive/2026-11-18-c3375，收缩版）：迁 sdk `provider/` 单一模块——trace / wire_policy / thinking 核心（compat 分派参数化，dialect 留 openai）/ json_parse / AdapterKind·AdapterRef（kernel 实测跨方言追加件）；attribution + reqwest_headers 整件留插件镜像（API 卫生：第三方方言作者不需要 courtesy 件）；删 hooks/tool_wire 2 壳；kernel 消费面直连 sdk；新方言脚手架成本 ~1005 行→0。实施随 T5 分组收拢。
- c3380 截断策略模块（已归档 archive/2026-11-18-c3380，组合式）：sdk truncation 纯函数引擎（Lines/Bytes/Both × Head/Tail，字符边界安全）+ generic_bytes_notice（mcp 现格式为统一模板）；bash 逐字脚注渲染器留 tools（r1431 钉死）；50 KiB 单一口径常数入 sdk，obs/codemode 独立常数不动，resource_cache 复用引擎；r1140 修 spec 对齐纯字节滚动尾窗（主语去类型化，直接编辑）；迁移五步 bash 不动；独立 propose。文本路线消掉「两套通知」中可统一的一半，spec 零变更。
- c3385 dispatch 合约归属（已归档 archive/2026-11-18-c3385，kernel 同轮全收敛）：sdk 持翻译合约本体（DispatchOutcome / SessionCommandExecutor / dispatch() / validate / transport_variant_error / variant_name，经 sdk→protocol 边引用 Command）；错误 = sdk DispatchRejection 小类型 + 两端 From wrap（XyDriverError 本体留两端）；5 数据类型下沉；app-tui 本地 trait 删（W14）；transport-only 名单三份→单点；新增 Command 3+2→1+1；独立 propose。
- c3390 lsp/dap 生命周期组件（已归档 archive/2026-11-18-c3390）：sdk agent_lifecycle 模块——泛型 AgentPhase&lt;H&gt; + 5 needle 共享文本分类器 + LazyAgent ensure 包装；lsp 保留 chain downcast 前置并组合共享分类器，dap 直接共享（语义不变）；schema_for 留各插件；承接 c3300 相位统一定义不重开 ADR；分类器/相位机 sdk 单测脱离真实 server；quick。
- T5 实施分组（已定 2026-11-18）：**四条独立变更线**——c3375 propose（provider 收敛 + 「Provider 与消息」句消歧）、c3380 propose（truncation 模块 + r1140 直接编辑）、c3385 propose（dispatch 合约 + sdk→protocol Cargo 边）、c3390 quick（agent_lifecycle）。四域 sdk 文件零交叠，可并行任意顺序；唯一共享点 sdk Cargo.toml 仅 c3385 触碰，执行时序放最后单独处理。

## Not yet specified（尚未清晰区）

（已清零——实施分组由 T5 决策关闭；thinking/wire_policy 近似重复随 c3375 Q1 收敛；新方言成本基线在 c3375 Q3 定为 ~1005 行 → 0。）

## Out of scope（范围外）

- 架构审查其余候选（Strong S1–S8、W5–W19、Speculative 各项）不进本图；S5/W7 与 S7 已由并行会话另行实施（286c34d、c3349），不在此重开。
- 截断常数的具体取值调整（属产品决策，非本图）。
