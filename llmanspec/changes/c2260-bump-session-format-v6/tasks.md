# Tasks: c2260-bump-session-format-v6

## 0. 前置

- [x] 0.1 确认 `c2250-harden-session-write-path` 已 finalize / archive（本票 depends_on 它；其落盘路径修复是本票 version bump 的安全前提）

## 1. Branch binding 与 Specs landing

- [x] 1.1 [blocked-by: 0.1] 前置：本规划壳已提交到默认分支，工作树干净；然后 `llman sdd change start c2260-bump-session-format-v6`
- [x] 1.2 [blocked-by: 1.1] 按 design §5 更新 `llmanspec/specs/agent-session-store/spec.toon`：s18 / s20 版本号 → 6；新增盘面时间戳条款 s22 + 非执行场景行
- [x] 1.3 [blocked-by: 1.1] `llman sdd context --task "session 盘面时间戳统一为 unix-ms、删除 serde alias、版本 6" --paths "src/protocol/session,src/protocol/message.rs,packages/xylitol-ai-bridge/src/dto/message.rs"`；复核 `domain-message` / `agent-session` 等候选 spec 是否有版本 / 时间戳硬编码条款，有则同步
- [x] 1.4 [blocked-by: 1.2, 1.3] `llman sdd validate c2260-bump-session-format-v6 --strict --no-interactive`；`llman sdd show c2260-bump-session-format-v6 --json` 确认 `readyToImplement=true`

## 2. schema v6：版本 + 时间戳

- [x] 2.1 [blocked-by: 1.4] `src/protocol/session/entries.rs`：`SESSION_VERSION` 5→6 + 版本史注释补 v6；`EntryBase.timestamp` / `SessionHeader.timestamp` `String`→`u64`
- [x] 2.2 [blocked-by: 2.1] 写点迁移：`inject_ids` / `clone_entry_with_ids` / `create()` header 改 `now_ms()`（同批共享一次取值）；`rfc3339_now` 无剩余调用即删
- [x] 2.3 [blocked-by: 2.1] 读点迁移：`list_sessions` 删 RFC3339 解析、ms 直读（mtime 回退保留）；HTML 导出标题展示边格式化（`time` crate）；`rg '\.timestamp\b' src/ packages/xylitol-ai-bridge/src/` 扫尾确认无字符串用法残留
- [x] 2.4 [blocked-by: 2.2, 2.3] 单测 / 夹具：v6 round-trip；v5 文件拒绝且文案含 `require 6`；手写 RFC3339 夹具（`session_export.rs:246,257` 等）改 ms 字面量；`list_sessions` 排序与 mtime 回退行为不回归

## 3. alias 清零

- [x] 3.1 [blocked-by: 2.4] 删 `src/protocol/message.rs` EnvMessage 9 个 snake alias + 「accept pre-fix JSONL」注释
- [x] 3.2 [blocked-by: 2.4] 删 bridge `packages/xylitol-ai-bridge/src/dto/message.rs` 11 个 alias + 对应注释；`just test` 全量确认 provider 适配器无断链（有断链在 adapter 显式映射，不恢复 alias）
- [x] 3.3 [blocked-by: 3.1, 3.2] 回归单测：v6 手工构造含 snake 字段的行按缺失 / 默认处理，不复活旧语义（s18 对齐验证）

## 4. streamTiming 形状锁定

- [x] 4.1 [blocked-by: 3.3] 形状锁定测试（design §4 三断言：注入键存在且类型正确 / 类型化反序列化成功 / 投影不受污染）；不改注入行为

## 5. 闸

- [x] 5.1 [blocked-by: 4.1] `just fmt` + `just lint` + `just test` + `just test-tui`；BDD feature 文件若有 version 5 / RFC3339 硬编码值一并更新
- [x] 5.2 [blocked-by: 5.1] `llman-sdd-verify` 出报告；全绿后 `llman sdd change finalize c2260-bump-session-format-v6`
