# Design — 会话存储重整为追加日志（breaking）

选型论证见 research/architecture-decisions.md（ADR-1 至 ADR-15），本文只写定稿结构。

## 0. 导读（新实现者入口）

本 change 不假设你读过任何对话记录；四份工件按序读即可独立实施。

阅读顺序：proposal.md（为什么改 / 改什么，10 分钟）→ 本 design（定稿结构与规则，§12 有逐行落盘示例）→ research/architecture-decisions.md（15 项选型论证；改任何一条规则前先读对应 ADR）→ research/implicit-state-machines.md（旧实现缺陷证据 S1..S10，附代码行号）。

术语表（历史词 → 本设计词）：

| 词 | 含义 |
|---|---|
| 压缩 compaction | 生成摘要替代旧史；新设计 = 追加一条 Compaction entry，零字节搬运 |
| 封存 seal（旧词，已废止） | 旧实现的物理搬家（写冷段/索引/新 active/manifest 四步事务）；新设计无此操作——旧文档里的 seal 指这个已废除动作，由「轮转 + 保留尾重发」取代 |
| 轮转 rotation | 写满或压缩点时切换到新 log 文件 |
| 保留尾重发 kept-tail re-emit | 轮转文件开头按原样字节重发 压缩行 + firstKept..leaf（同 id，newest-file-wins 去重） |
| 锚点 anchor | index 中每 64 行一条的 entry_id → (file, offset) 记录 |
| 兼容门 | 旧格式检测 → 显式拒绝（不做迁移） |

现状代码地图（将被本 change 重写替换；plugins/xylitol-official-plugin-session/src/manager/）：

| 文件 | 行数 | 职责 / 缺陷对应 |
|---|---|---|
| segments.rs | 595 | 段读写、write_file_atomically、next_generation 反推（S2）、current_active_path 静默回退 |
| persist.rs | 475 | append 主路径、pending 缓冲、assistant-only flush（S3） |
| load.rs | 471 | 分支加载、两套 leaf 回退顺序（S4） |
| write_lock.rs | 327 | flock / 信号量、「无 manifest 视为持锁」窗口（S6） |
| store.rs | 287 | XySessionStore port 实现（接口形状保留） |
| tree.rs | 273 | fork / travel / navigate |
| context.rs | 256 | thinking_level 扇扫（S8） |
| seal.rs | 187 | 压缩提交四步事务（S1） |
| mod.rs | 148 | SessionManager 聚合、leaf_ids 三态 |
| edges.rs | 123 | 图边读写（树视图沿用其语义） |

关键外部事实：

- 词表：SessionEntry serde tag = type、camelCase（crates/xylitol-plugin-sdk/src/session/entries.rs:210 起）；本 change 新增可选字段仅两处：Header 行 format、每行 seq。
- port：XySessionStore trait 在 crates/xylitol-plugin-sdk/src/ports/session.rs:33；消费方不得绕过（ADR-14）。

硬不变量（违反任何一条 = 实现错误）：

1. 单写者：一个 runtime 绑一个 session（src/AGENTS.md 会话 actor 条款）；跨进程互斥用 session.lock。
2. 正确性不依赖 manifest / index（二者是可丢弃缓存，坏即重建）。
3. 不引入 DB；不做旧格式迁移；不做自动重分片。
4. 投影确定性：同分支同 leaf 必产出相同投影（provider 前缀缓存依赖）。

交付判定（验收）：

- 投影等价：旧实现对既有场景的输出冻结为黄金集，新 store 逐一对照。
- 崩溃矩阵：append / 轮转 / 压缩 / 保留尾重发每一步 kill 后可恢复（§10）。
- BDD：agent-session-store 场景重写全绿。

## 1. 目标与边界

- 唯一真值 = log/ 下追加式 JSONL；manifest 与 index 全部是可丢弃派生缓存。
- 崩溃恢复两条机械规则；写路径唯一（append + fsync）；旧格式显式拒绝。
- 边界：不引入 DB（四触发条件未到）；不做自动重分片；不改 XySessionStore port 形状（语义微调：分页 token 化）。

## 2. 磁盘布局与读写路径

~~~text
~/.xylitol/sessions/<id>/
├── log/
│   ├── 000001.jsonl     只追加；行 = SessionEntry（词表不变，多一个 seq 可选字段）
│   └── 000002.jsonl     当前写头（manifest 记录）
├── manifest.json         缓存：{ format: "log-v1", active_file, tail_offset, leaf_id,
│                                  rotation: {policy, threshold_mb}, index: {built_at} }
├── index/
│   └── 000002.idx        稀疏锚点：每 64 entry 一条（entry_id -> file+offset）
└── session.lock          flock / 信号量载体，先于一切写创建
~~~

写入：append 一行 + fsync（粒度见 §8）→ 更新内存写头；manifest 延迟落盘（它只是缓存，丢了重建）。
读取（三视图见 §3）：LLM 视图从 leaf 走 parent 链；树视图全量；分页视图走 §7。

## 3. 投影模型（三视图 + 压缩双语义）

~~~mermaid
flowchart LR
  LOG["log/*.jsonl（真值，不可变）"] --> V1["LLM 视图：leaf→root 折叠<br/>碰 Compaction 截断（摘要作前缀）"]
  LOG --> V2["树视图：全量可浏览<br/>无视 Compaction"]
  LOG --> V3["分页视图：turn/entry 粒度窗口"]
  V1 --> CACHE["投影缓存 keyed by leaf_id<br/>追加后 O(delta) 折叠"]
~~~

- turn = 派生规则（user entry 开界），函数唯一实现。
- 确定性投影保证 provider prompt cache 命中；压缩是唯一断点（可解释）。

## 4. 轮转策略（双触发 + 自适应 + turn 对齐）

- 触发：当前文件大小 ≥ 自适应阈值，或压缩 entry 提交；二者都只在 turn 边界执行。
- 阈值：初始 64MB；维护任务按会话中位 turn 字节调整，范围 [16MB, 256MB]。
- 压缩点轮转语义（定稿）：压缩提交 = 在旧文件追加一条 Compaction entry（单行，天然原子）；轮转加速 = 新文件按原样字节重发 Compaction + first_kept..leaf 拷贝（同 id， newest-file-wins 去重），此后新 entry 追加进新文件。拷贝幂等、随时可崩（未挂载的新文件按孤儿清理，manifest 翻转是唯一挂载点）；LLM 视图正确性不依赖拷贝——没有新文件就沿旧文件链走，有则单文件快路径。
- 不做自动重分片（ADR-8）；repack 留显式命令逃生口。

## 5. 维护单元（lazy：打开快检 + CLI）

| 时机 | 动作 | 成本 |
|---|---|---|
| 会话打开（快检） | 尾部半行截断；.tmp 清理；manifest/index 新鲜度校验（过期则重建） | 毫秒级 |
| xylitol sessions maintain | 全量索引重建；孤儿文件清理；健康统计上报；阈值建议重算 | 秒级，幂等 |
| 不做 | 常驻调度、自动重分片、压缩触发、DB 升移 | — |

## 6. 索引与分页

- 稀疏锚点（每 64 entry）：entry_id → (file_no, byte_offset)；定位 = 锚点 + ≤63 条线性扫。已决（2026-11-11）：保留磁盘锚点形态；index 缺失 / 损坏 / 未建时读路径回退线性扫描（正确性不依赖索引），重建归维护任务。
- 尾部分页（leaf 在尾部，99% 场景）：从写头文件 EOF 向前按块（256KB）倒扫，数 user entry 到 turn 边界。
- page token = { format_version, leaf_id, anchor_entry_id, direction, unit(turn|entry), limit }；失效 → 从 leaf 重开。

## 7. 崩溃恢复与 fsync

- 规则一：尾部 JSON 解析失败的半行 → 截断（Kafka / SQLite 同款）。
- 规则二：manifest / index 损坏或缺失 → 从 log 重建；绝不当作空会话。
- fsync 粒度 = turn 边界批量（已定稿 2026-11-11：一个 turn 一批 + 进程收尾批；崩溃最多丢正在进行的 turn，截断规则兜底）。

## 8. 兼容门（breaking）

- 打开旧布局（v7 manifest / legacy 单文件）→ XySessionStoreError::unsupported，文案：「session 格式已升级且不兼容，请开新 session；旧数据可用导出工具抢救（后续提供）」。
- 新格式第一行 Header 内嵌 format: log-v1；后续演进走可选字段，不再 breaking。

## 9. 写锁

- flock（POSIX）/ 命名信号量（Windows），复用现有平台抽象；锁文件先于 log 目录任何写创建；进程消亡自动释放（无死锁文件）。

## 10. 测试边界（seam）

- 单测：崩溃矩阵（append / 轮转 / 压缩 / 截断各点 kill）；投影等价（与现 build_context 对照）；分页边界（跨文件、token 失效）；双进程写互斥。
- BDD：agent-session-store 场景重写（36 requirement 大部分改写）；旧格式拒绝门；最后 3 turn 懒加载。

## 11. 风险与回滚

- 最大风险：manager 重写引入新 bug 面 → 投影等价测试先行（旧实现输出冻结为黄金集）。
- 回滚：整体 revert（breaking 无渐进面）；新格式数据在回滚后不可读——接受（未推广期）。

## 12. 附录：落地文件参考示例

示例会话 s_9f2c81（一次压缩后、两个 log 文件）。entry 包装字段与现词表一字不差（serde tag = type，camelCase），新增仅两处可选字段：Header 行的 format、每行的 seq。

~~~text
~/.xylitol/sessions/s_9f2c81/
├── log/000001.jsonl   10 行：header + turn1(4 行) + turn2(4 行) + 压缩提交行
├── log/000002.jsonl    7 行：压缩行+保留尾重发(5 行) + turn3 新增
├── manifest.json       缓存
├── index/000001.idx    稀疏锚点
├── index/000002.idx
└── session.lock        flock 载体（可写 pid 供人看，真值是 OS 锁）
~~~

### log/000001.jsonl

~~~text
{"type":"session","id":"s_9f2c81","timestamp":1762830000000,"cwd":"/Users/l8ng/Projects/demo","format":"log-v1"}
{"type":"message","seq":2,"id":"e_01","parentId":null,"timestamp":1762830001000,"message":{…user…}}
{"type":"message","seq":3,"id":"e_02","parentId":"e_01","timestamp":1762830004000,"message":{…assistant+tool_call…}}
{"type":"message","seq":4,"id":"e_03","parentId":"e_02","timestamp":1762830006000,"message":{…tool result…}}
{"type":"message","seq":5,"id":"e_04","parentId":"e_03","timestamp":1762830009000,"message":{…assistant text…}}
{"type":"message","seq":6,"id":"e_05","parentId":"e_04","timestamp":1762830100000,"message":{…user（turn2 起）…}}
{"type":"message","seq":7,"id":"e_06","parentId":"e_05","timestamp":1762830103000,"message":{…assistant+tool_call…}}
{"type":"message","seq":8,"id":"e_07","parentId":"e_06","timestamp":1762830105000,"message":{…tool result…}}
{"type":"message","seq":9,"id":"e_08","parentId":"e_07","timestamp":1762830108000,"message":{…assistant text…}}
{"type":"compaction","seq":10,"id":"e_09","parentId":"e_08","timestamp":1762830110000,"summary":"…turn1 摘要…","firstKeptEntryId":"e_05","tokensBefore":18432}
~~~

压缩提交 = 第 10 行这一条 append（单行原子）。firstKeptEntryId=e_05：turn1 被摘要替代，turn2 保留。

### log/000002.jsonl（压缩点轮转：保留尾重发）

~~~text
{"type":"compaction","seq":10,"id":"e_09",…}        ← 字节级重发（同 id）
{"type":"message","seq":6,"id":"e_05",…}            ← 保留尾重发（同 id，链不变）
{"type":"message","seq":7,"id":"e_06",…}
{"type":"message","seq":8,"id":"e_07",…}
{"type":"message","seq":9,"id":"e_08",…}
{"type":"message","seq":11,"id":"e_10","parentId":"e_09","timestamp":1762830200000,"message":{…user（turn3 起）…}}
{"type":"message","seq":12,"id":"e_11","parentId":"e_10","timestamp":1762830204000,"message":{…assistant…}}
~~~

- 去重规则：同 id 多文件出现时 newest-file-wins（查表先新后旧）。
- LLM 视图链：e_11→e_10→e_09(压缩)→e_08→…→e_05(firstKept 停) —— 全部命中 000002，单文件快路径。
- 树视图：e_05.parentId=e_04 落在 000001，回看穿透。
- 崩溃安全：重发幂等；manifest 未翻转前的新文件 = 孤儿（打开快检清理）；重挂载后重新对账补拷。

### manifest.json（缓存，可丢弃重建）

~~~json
{"format":"log-v1","sessionId":"s_9f2c81","activeFile":2,"tailBytes":4721,"leafId":"e_11",
 "rotation":{"policy":"adaptive-turn-aligned","thresholdBytes":67108864,"minBytes":16777216,"maxBytes":268435456},
 "index":{"anchorEvery":64,"builtAt":1762834321,"files":[1,2]},"updatedAt":1762834321}
~~~

### index/000002.idx（稀疏锚点，每 64 行一条）

~~~text
{"seq":1,"entryId":"e_09","offset":0}
（下一条锚点 seq=65）
~~~

### page token（分页游标）

明文 {"v":"log-v1","leaf":"e_11","anchor":"e_06","dir":"older","unit":"turn","limit":3}；传输 = base64url，客户端当不透明 cookie，失效（版本不符）→ 从 leaf 重开。

### 「最后 3 turn」走查

manifest → activeFile=2 → 从 EOF 倒扫 256KB 块 → 数 user 行：e_10 一个、e_05（重发）两个 → turn 边界 e_05 → 窗口 e_05..e_11 共 7 行 → 单文件完成，未读 000001。窗口跨界时续读上一文件尾部（或经 idx 锚点跳转）。
