# Design

## 裁决（逐条，对应 research/spec-org-audit-2026-08-22.md）

| req | 裁决 | 新语义 |
|---|---|---|
| agent-session-store sp1+sp2 | **合并为一条 sp1**（端口 seam 例外保留方向，去模块路径前缀） | infra 层 SessionManager MUST 实现 protocol 的 XySessionStore 端口（append/load/load_context/exists），仅覆盖 agent 循环与 compaction 所需；完整面（fork/navigate/export）MAY 留在具体结构体供组合根直接使用。sp2 删除；feature 中两个同名 `manager-impls-store` 场景合一 |
| agent-tools t18 | **去迁移史 + 去字段签名**，保留层方向例外 | ToolSet MUST 作为构建期终态编排状态留在 agent 层：聚合具体工具并组合单元操作，构建后交给一轮的集合即终态，无运行时过滤；具体工具由组合根注入 |
| test-provider-integration cv3 | **去实现载体钉** | ConfigValueResolver 对 `!` 前缀命令 MUST 以 10 秒超时执行并在进程生命周期内缓存结果；超时以失败结果呈现 |
| infra fn 签名族（g1/g3/p1/p2/p3/p4/i5/r18/r19/rd10/rd11/rd12 等） | **去函数名/签名括号**，保留能力 + 可观察返回 | 如「向上遍历定位 repo root 并区分 worktree」「PATH 注入 bin 目录」「按 MIME 推导扩展名写 tempfile」等 |

## 边界

- 不改任何行为；纯文本合约瘦身。
- 未绑定 `.feature` 的场景文字同步改写以防漂移；bound feature 不涉及本票条款。
- borderline 保留项（qa-gate / tui-testing / paste-burst Instant 等）不动，理由见 research。

## 验证

strict validate（五 capability + change）；全量 validate 含 BDD——BDD 不触这些文档行，应保持绿。
