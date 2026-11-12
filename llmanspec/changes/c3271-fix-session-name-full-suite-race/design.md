# Design — 会话名全量套件竞态（全部已决 2026-11-12）

## 0. 嫌疑链（按证据强度）

~~~mermaid
flowchart TB
  A["set_session_name（sdk 默认实现）<br/>SessionInfoEntry{id:""}"] -->|append_session_entry with_id=false| B["log 落盘：无 id 行"]
  B --> C["persisted_load_all 投影<br/>map.insert 仅收带 id 行（project.rs:76-79）"]
  C -->|无 id 行被丢弃| D["get_session_name 扫不到 SessionInfo → None"]
  E["触发条件：仅全量套件并发<br/>（单测/满载/子集均不复现）"] -.-> B
~~~

未解之谜（诚实记录）：若嫌疑链完全成立应确定性失败而非概率性——并发环境如何介入尚无定论（候选：tokio fs 读取时序 × 进程风暴下的页缓存行为）。修复策略因此选「消灭无 id 行」这一根因级改法，无论并发介入机制为何均成立。

## 1. 修复

1. `log/store.rs` `append_session_entry`：`append(self, id, entry, false)` → `true`（trait 口全部条目带 id）。
2. `log/project.rs` 投影防御：无 id 行不再静默丢弃——按 seq 插入稳定位置（防未来再出现无 id 写入方）。
3. 存量数据：已落盘的无 id 行由防御路径 2 收纳，无需迁移。

## 2. 验收

- 单测：set→get 立即读回（新增，钉住本 bug）。
- 全量 `just test` 连续 5 次零失败（历史 ~50% 失败率下，5 连绿置信 >97%）。
