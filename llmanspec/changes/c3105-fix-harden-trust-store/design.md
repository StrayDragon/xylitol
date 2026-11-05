# Design — 加固 trust store

## 1. 存储格式

`trust.json` 采用 schema v2：

```json
{
  "version": 2,
  "entries": { "/canonical/path": true, "/other": false },
  "hmac": "<hex>"
}
```

- `entries` 为规范绝对目录到 `true` / `false` 的映射；清除决策 = 删除键。
- `hmac` 为 HMAC-SHA256：密钥来自 `<agent_dir>/trust.key`，输入为 `version\n` + `entries` 的稳定 JSON（BTreeMap 序列化，键有序）。
- 旧格式（裸 map）在读取时识别并迁移为 v2；迁移保留全部决策。
- HMAC 校验失败：视为 `trust_tamper`，内存中按空 store 处理（所有项目回到未决），不得静默信任，也不得覆盖现场文件。

## 2. 密钥

- 路径 `<agent_dir>/trust.key`，内容为 64 位 hex（32 字节随机），权限 0600。
- 首次写 store 时创建；创建走原子写（tmp + rename）。
- 密钥缺失但 store 为 v2（带 hmac）：视为篡改，空 store + `trust_tamper`；不得自动重建后继续信任。
- 密钥缺失且 store 为旧格式：迁移时创建密钥并重写 store。
- 密钥与 store 同在 agent dir；同用户进程可读，因此 HMAC 只提高改写门槛，不构成安全边界。

## 3. 写盘与权限

- 原子写：同目录 tmp（`trust.json.tmp`）以 0600 创建，写入并 `sync_all`，再 `rename` 到 `trust.json`。
- `trust.json`、`trust.json.lock`、`trust.key` 均为 0600；目录保持现有权限。
- 读取时若发现 `trust.json` 权限宽于 0600（unix），就地 chmod 0600 并记 `trust_store_write`（reason = `migrate_permissions`）。
- 非 unix 平台跳过权限位操作，仍保持原子写。

## 4. 迁移

- 读取顺序：不存在 → 空；解析 v2 → 校验 HMAC；解析旧格式 → 迁移；解析失败 → 视为损坏，空 store + `trust_tamper`。
- 迁移写盘一次，事件 `trust_store_write`，reason = `migrate_schema` 或 `migrate_permissions`。
- 迁移失败返回 `XyTrustError`，调用方按未决处理；不得以旧文件继续信任。

## 5. 审计

- 落点 `<agent_dir>/audit/trust.log`，0600，JSONL，append-only。
- 事件：`trust_grant` / `trust_deny` / `trust_revoke` / `trust_tamper` / `trust_recheck` / `trust_store_write`。
- 字段：`ts`（RFC3339）、`event`、`actor`（tui / host / cli / system）、`session_id`、`workspace`、`decision`、`source`（tui_prompt / host_unary / resume / startup）、`hmac_ok`、`reason`。
- 轮转：写入前若文件将超过 `audit_max_bytes`（默认 10 MiB），`trust.log` → `trust.log.1` … 顺延，最多保留 `audit_keep_files`（默认 5）；轮转时删除 mtime 早于 `audit_retention_days`（默认 90）的归档。
- 审计写失败只记 `log::warn`，不得改变 trust 决策或写盘结果。
- 不记录 secrets；路径为 canonical path。

## 6. 配置

新增 `trust` 配置节：

```yaml
trust:
  audit: true            # 默认 true
  audit_max_bytes: 10485760
  audit_keep_files: 5
  audit_retention_days: 90
```

- `trust.audit` 为 false 时不写审计（仍保持 0600 / HMAC / 迁移）。
- 配置节由 config 插件声明 schema；runtime 只消费纯 `TrustAuditConfig`，不依赖 config 插件。

## 7. 人类确认入口

- trust 决策只允许人类操作：CLI/TUI 交互选择；host unary（`trust_cwd` / `trust_parent` / `deny`）。
- 不注册任何 LLM 可调用工具；不得把 trust 决策暴露为 MCP / agent 工具。
- host unary 仍要求已认证 client（现有 Basic / token 门禁）；本 change 不新增 capability 协商，只把审计来源标为 `host_unary`。

## 8. resume 重校验

- resume / materialize writer 时按 workspace 重新解析 trust（不直接复用进程启动缓存）。
- 每次 resume 记 `trust_recheck`（actor = host，source = resume，hmac_ok 反映 store 校验结果）。
- workspace set 的逐 workspace 校验在 c3050 落地；本 change 只保证 primary workspace 的 resume 重校验路径存在。

## 9. Advisory 边界

- 文档与 spec 明示：trust store 是 advisory，不是安全边界；同用户任意代码可改 store/key/audit。
- 真隔离依赖 OS sandbox / 独立用户 / 容器；不得把 HMAC 描述为防同用户攻击。

## 10. 失败策略

- store 损坏 / HMAC 不匹配：空 store（未决）+ tamper 审计；交互路径重新询问。
- 锁获取失败：返回 `XyTrustError::Lock`，调用方按未决处理。
- 审计目录不可写：warn，不影响决策。

## 11. 测试

- 单测：0600 权限、原子写、旧格式迁移、0644 迁移、HMAC 篡改、密钥缺失、审计事件与字段、轮转、配置解析、resume recheck。
- BDD（domain-security）：store 加固、tamper 拒绝、审计事件、advisory 边界。
- runtime-config：`trust.audit` 配置键解析与默认值。
