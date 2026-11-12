# Design — 会话名全量套件竞态（全部已决 2026-11-12；§0 实施首日复核改写）

## 0. 复核结论（T1，2026-11-12）

原嫌疑链复核后**不成立**：

- `log/append.rs:55` `prepare()` 在 `with_id=false` 时把 `base.id` **覆盖为新 uuid**（不是留空）；`with_id=true` 才保留调用方 id、只补空字段。经 trait 口落盘的行始终带 id，故「无 id 行 → 投影丢弃」不成立。
- 无 id 行即使出现也不会被丢弃：`entry_id()` 对空 id 返回 `Some("")`，`project.rs` 的 `map.insert("")` 会**折叠**全部无 id 行。本次改为按 seq 收纳（防御）。

真实根因（已用确定性单测复现；修复前红、修复后绿）：

~~~mermaid
flowchart TB
  A["materialize_writer 绑定会话"] -->|start_automation_if_needed| B["automation_loop：interval 首个 tick 立即触发"]
  B --> C["AutomationEngine::tick → store.load_leaf_branch → load_all"]
  C --> D["quick_check → truncate_partial_tail（读路径改写日志）"]
  E["append_persisted：write_all(行) 与 write_all(\n) 是两次阻塞池往返"] --> F["两写之间存在让出窗口"]
  F -->|读者落在窗口内| D
  D -->|文件尾无换行| G["截到上一个换行：正在写入的完整行被丢弃"]
  G --> H["GetSessionName 扫不到 SessionInfo → None"]
~~~

- 读路径 `quick_check` 每次 `load_all` 都调用 `truncate_partial_tail`；该函数只要文件尾无换行就截断，不判断尾部是否是完整 JSON。
- `append_persisted` 的「行」与「换行」是两次 `write_all`，各为一次阻塞池往返，两次之间存在真实让出窗口。
- `materialize_writer` 末尾 `start_automation_if_needed` 启动 `automation_loop`；`tokio::time::interval` 首个 tick 立即触发，`AutomationEngine::tick` 走 `load_leaf_branch` → `load_all`。SetSessionName 写入前后恰好有一个同进程并发读者。
- 全量套件并发改变调度让出点，故 ~50% 概率；单测/子集调度稳定，故 100 次零失败。

## 1. 修复

1. **读路径只读**：`quick_check` 不再截尾；截尾/修复只在持写锁的写路径（`append_persisted`）与维护命令（`maintain_session`，拿不到写锁则只读巡检）执行。
2. **写路径单次写入**：`append_persisted` 用一次 `write_all` 写「行+换行」，消灭两写窗口。
3. **修复分级**：完整 JSON 尾只补换行（保数据）；损坏尾才截断（并 `log::warn!`）。
4. 决策项照旧：`append_session_entry` 统一 `with_id=true`；投影对无 id 行按 seq 收纳（防御未来写入方）。
5. 存量数据：已落盘的无 id 行由防御路径收纳，无需迁移。

## 2. 验收

- 单测（钉住本 bug）：`load_all` 不得截断完整在写尾；set→get 立即读回；并发读者下 40 次 append 零丢失；无 id 行按 seq 收纳。
- 全量 `just test` 连续 5 次零失败（历史 ~50% 失败率下，5 连绿置信 >97%）。
