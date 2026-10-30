# Design：裸规则转场景与覆盖源固定

## 判定

- **转场景**：规则描述可观察行为，且未声明已由单测/门禁覆盖。
- **保留裸规则**：结构性不变量、crate 图/门禁约束、或正文已声明“由 … 覆盖 / 不单独扩 BDD step”。
- 新增场景只作为契约示例；不新增 `#[scenario]` binding，避免与现有 BDD 步词表冲突。

## 转场景（28）

| capability | req id |
|---|---|
| agent-plan | r2948 / r2949 / r2950 / r2951 / r2974 |
| agent-prompt | r2915 |
| agent-runtime | r2955 / r2937 / r2939 / r2940 / r2942 / r2943 / r2944 |
| agent-session | r1083 / r1084 |
| agent-session-store | r2938 / r2941 / r2952 / r2945 |
| agent-workflow | r2946 |
| app-tui-bridge | r2957 |
| app-tui-commands | r2954 |
| app-tui-fixed-zone | r2956 / r2958 |
| infra-observability | r3006 |
| protocol-app | r2947 |
| runtime-config | r2959 |
| server-core | r2973 |

## 保留裸规则并补覆盖源（18）

| capability | req id | 覆盖源 |
|---|---|---|
| agent-llm-projection | r2968 | workflow 插件单测（已有） |
| agent-runtime | r2962 | 单测（已有） |
| agent-session | r1077 | crate 图与端口测试 |
| agent-session | r1080 | trust store 与端侧应用面单测 |
| agent-session-store | r1115 | SessionStore 端口契约单测 |
| agent-session-store | r2964 | plan 插件单测（已有） |
| agent-session-store | r2967 | 插件单测（已有） |
| agent-session-store | r2970 | SDK 单测（已有） |
| agent-tools | r2961 | plan 插件单测（已有） |
| app-tui | r2972 | host 单测（已有） |
| app-tui-host | r3005 | harness 单测（已有） |
| domain-compaction | r1400 | 配置类型与 serde 单测 |
| domain-security | r2965 | plan 插件单测（已有） |
| infra-observability | r3003 | kernel ErrorSink 单测 |
| layer-architecture | r1535 | 类型与组合根（已有） |
| runtime-config | r1740 | 配置边界 JsonSchema 快照/单测 |
| test-provider-integration | r1825 | crate 图与 provider 装配单测 |
| workspace-assembly | r2916 | crate 图与层门禁（已有） |

## 验证

- `llman-sdd validate --specs --strict`
- `llman-sdd review`（unbound 46 → 18）
- `just qa`
