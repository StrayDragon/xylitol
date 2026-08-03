# Tasks: c1870-add-otel-llm-infra-split

## 1. 合约与文档对齐

- [x] 1.1 `infra-otel`：收紧 otel19「实际执行」= 过 prepare；新增 `xylitol.obs.lane` / 直连降噪 req；scenarios 保持 `feature: false`
- [x] 1.2 （跳过）`domain-compaction` 已有 prepare 语义，无需重复
- [x] 1.3 architecture「理想 vs 现状」更新本波已兑现行；roadmap 仍列 Collector 运维 / infra span 候补

## 2. 发射：lane + prepare-first

- [x] 2.1 统一 helper：`langfuse_observation_properties` / generation 带 `xylitol.obs.lane=llm`；token.estimate 同打标
- [x] 2.2 `CompactionOrchestrator::compact`：prepare-first；Ok 后 span；Err 不建 OTLP
- [x] 2.3 auto 路径经同一 span helper 继承 lane
- [x] 2.4 CollectingReporter 单测：lane 断言；独立根改为 post-prepare 失败语义

## 3. 运维示例

- [x] 3.1 `configs/examples/otel-collector-lane.yaml` + `configs/example.yaml` 指针
- [x] 3.2 注释：默认仍可直连 Langfuse；infra 时改 endpoint

## 4. 校验

- [x] 4.1 `llman sdd validate c1870-add-otel-llm-infra-split --strict --no-check`
- [x] 4.2 相关 lib 单测 + `just fmt` / clippy 触及面
