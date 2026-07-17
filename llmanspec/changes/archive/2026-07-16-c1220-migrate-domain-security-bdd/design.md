# Design — c1220-migrate-domain-security-bdd

## 决策

1. **modify_requirement 而非 add_requirement**：r7 已存在于 main spec，本变更只给它补
   可执行场景（op_scenario + feature:true），不改其语义。solidify 把 op_scenario 合并生成
   .feature。

2. **纯文本 step（无占位符）**：rstest-bdd 的 `{domain:string}` 占位符期望引号界定的值
   （现有 sandbox.feature 写 `检查网络域名 "evil.com"`），但 solidify 原样输出 spec.toon
   字段（不加引号）。两条路二选一：
   - (a) spec.toon 字段内嵌引号 → TOON 转义复杂、易错；
   - (b) step 用纯文本（`检查网络域名 evil.com`，无占位符）→ 干净，但牺牲参数化。
   选 (b)。迁移场景多为固定值断言，参数化收益低；纯文本最稳。

3. **复用 sandbox_bdd mod**：新 step 加在现有 `mod sandbox_bdd`（bdd.rs 2268+）内，
   共享 thread_local SANDBOX_ENGINE / LAST_VERDICT，复用 default_sandbox()（已含
   blocked_domains=["evil.com"]）。

## 验证证据

- `cargo test --test bdd` → 100 passed（99 + 1），0 回归
- `llman sdd validate c1220 --strict` 通过
- solidify 生成的 .feature 与 spec.toon delta 字节级一致
