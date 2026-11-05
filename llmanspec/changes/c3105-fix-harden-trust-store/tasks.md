# Tasks — 加固 trust store

- [x] 1. TrustManager 存储加固：v2 schema + HMAC + 0600 原子写 + lock 0600 + 旧格式/权限迁移；单测覆盖权限、迁移、篡改、密钥缺失。
- [x] 2. 审计 JSONL：事件、字段、轮转、保留期；审计失败不影响决策；单测覆盖事件与轮转。
- [x] 3. 配置接线：`trust` 配置节（audit / 轮转 / 保留期）+ runtime `TrustAuditConfig`；config 解析单测。
- [x] 4. 决策接线与 resume 重校验：bootstrap / driver persist / host recheck 带 actor/source 审计；host resume 重新解析 primary workspace trust；单测覆盖 recheck。
- [x] 5. Specs 与 BDD：domain-security 加固/审计/advisory 条款、runtime-config `trust.audit`、server-core resume 重校验、app-tui-trust 人类确认；绑定 BDD 场景。
- [x] 6. 门禁：`just fmt` / `just lint` / 相关测试 / `llman-sdd validate c3105-fix-harden-trust-store --strict` / `just qa`。
