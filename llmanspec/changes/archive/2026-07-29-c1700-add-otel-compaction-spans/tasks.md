# Tasks: c1700-add-otel-compaction-spans

## 0. Hygiene

- [x] 0.1 修订 live `test-bdd`：`valid_scope` 将 `tests/bdd.rs` 改为 `tests/bdd/`；tb3 statement 中路径措辞同步（不改 BDD 行为）

## 1. Specs

- [x] 1.1 修订 live `infra-otel`：扩展 `otel8`/`otel12`（含 `agent.compaction` + type=`span`）；新增 `otel19`（父子挂载、reason/will_retry/关闸、默认无摘要 I/O）；`.feature` 同步 otel6/8/11/12；`otel19` 以 `feature: false` + 单测为准
- [x] 1.2 修订 live `infra-observability` `ipt4`：低频 span 列表纳入 `agent.compaction`（挂 turn / 禁止活跃 turn 下无关 random 根）

## 2. Implementation

- [x] 2.1 增加 `AgentCompactionSpan`（或等价）helper：闸、`langfuse_observation_properties("span")`、turn/独立根 parent、lifecycle events；与 `token.estimate` 同闸
- [x] 2.2 在 `CompactionOrchestrator` force/auto（含 overflow）路径包住 Start→compact→End；结束写 reason/`will_retry`/`aborted`/error；尽量把摘要 `llm.request` parent 指到该 span
- [x] 2.3 CollectingReporter 单测：挂 turn、独立根+session、关闸 noop、reason 诚实

## 3. Docs + gate

- [x] 3.1 更新 `docs/architecture/进程内观测.md`（及必要时 `压缩与上下文.md`）过程树：含 `agent.compaction`；不写易腐 change id
- [x] 3.2 `llman sdd validate c1700-add-otel-compaction-spans --strict`；相关 lib 单测绿
