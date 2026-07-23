# Design: c1540-update-tool-timeout-unlimited

## Goal

工具执行超时默认 **Unlimited**；仅显式正数秒启用 timer。类型有语义，不用魔法 `0`。

## Types

```rust
enum ToolTimeout {
    Unlimited,
    After(Duration),
}

impl ToolTimeout {
    /// `None` → Unlimited；`Some(0)` / 负数 → Err；超 max → Err 或 clamp（钉：Err）
    fn from_secs_opt(secs: Option<u64>) -> Result<Self, …>;
}
```

- LLM JSON：`timeout` 字段 **optional**；省略 = Unlimited
- Hook 配置：`timeout_secs: Option<u64>`（serde 缺省 `None`）
- 内部执行：`match timeout { Unlimited => /* 不设 sleep/timeout future */ ; After(d) => … }`

## Max bound

- bash / grep / find：显式 timeout **MUST** 为 `1..=MAX_TIMEOUT_SECS`（建议 `MAX_TIMEOUT_SECS = 86400`（1 天）或保留现 bash `120`——**实施取 86400 对齐「长任务可显式限时」；若担心 LLM 乱传，可仍用 120 并在 schema description 写明**）
- **决策**：`MAX_TIMEOUT_SECS = 120` 先保持与现 bash schema 一致，降低行为惊吓面；需要更长时另开 change
- Hook：有值时 `1..=` 同级上界或与 bash 共用常量；无「至少 1」的 `max(1)` 对 `None` 的误用

## Path unification

| 路径 | 行为 |
|---|---|
| `BashTool` 非流式 `RealBashOperations` | 尊重 `ToolTimeout`；`After` → graduated SIGTERM→SIGKILL |
| `BashTool` 流式 → `InfraBashExecutor` | **同一** `ToolTimeout` 传入；禁止丢弃 / 硬编码 30s |
| bang `XyBashExecutor` | 默认 Unlimited；若端口需可选 timeout，经 `BashExecOpts` 扩展（本 change 可加 `timeout: ToolTimeout`） |
| grep / find | 可选 args；`After` 时超时杀子进程并 `XyToolError::Timeout` |
| Hook dispatcher | `None` = 不包 `timeout()`；`Some(n)` = 到期杀脚本 |

超时后既有逐级升级（infra-bash `b3`）仅在 **有限时** 路径生效。

## Test seams

- BDD：`agent-tools.feature`（bash 显式 timeout / 非法 timeout；grep/find 可选 timeout）
- BDD：`agent-hooks.feature`（显式 1s 杀脚本）
- BDD / 单测：`infra-bash` 有限时逐级升级（既有 `b3`）
- 单测：省略 timeout → `ToolTimeout::Unlimited`；流式路径转发 timeout；`0` 拒绝

不可执行（过慢）：「默认无限可跑 >30s」用单测断言未武装 timer，或 feature:false 文档场景。
