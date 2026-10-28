# Design

## 配置

```rust
pub struct SelfHealConfig {
    pub enabled: bool,
    pub signatures: Vec<String>,
    pub max_attempts: u32,
}
```

- `signatures` 为空时用内置确定性失败签名（测试失败、编译错误、lint 失败、panic 等）；
- `max_attempts` 缺省 3，必须远小于 core 的 32。

## 中间件

`SelfHealMiddleware`：order 210，阶段 `after_tool` + `decide_continue`。

```text
after_tool:
  is_error 且文本命中签名
    → pending = { tool, signature, attempts+1 }
    → 在结果后追加短 <self_heal> 指引（诊断→patch→重跑同一验证）
  成功且工具名与 pending.tool 相同
    → 清除 pending（视为验证通过）

decide_continue:
  pending 且 attempts < cap → Continue（core 再调度一次模型调用）
  pending 且 attempts >= cap → Stop（稳定结束，不无限自愈）
  无 pending → Continue（不干预基线）
```

`<self_heal>` 指引只写进工具结果，不改 `is_error`；模型下一轮能看到失败签名与「不要原地重试」的要求。

## 确定性 vs 非确定性

只有命中签名才进入自愈。超时、网络抖动、权限拒绝等默认不命中，直接交回模型或由 core 结束。

## 有界与升级

- 每个失败签名/tool 组合独立计数；不同失败重新从 1 开始。
- 达到 cap 后 `decide_continue` 返回 Stop，并在日志记录签名与次数。
- 不自动 VCS 回滚；只要求模型回退或说明。

## 与 plan-state 的关系

两者可同时启用。`decide_continue` 的 Stop 由**最先返回 Stop 的中间件**决定；自愈只负责在 cap 内强制续跑，不覆盖其他中间件的停止语义。若计划中间件先停止，run 仍结束。

## 不做什么

- 不在插件内调用模型或工具。
- 不做自动回滚 / 事务。
- 不把非确定性失败纳入自愈。
- 不改 core 的 32 次硬上限。
