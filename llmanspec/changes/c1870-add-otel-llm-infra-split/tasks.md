# Tasks: c1870-add-otel-llm-infra-split

## 1. 合约与文档对齐

- [ ] 1.1 `infra-otel`：收紧 otel19「实际执行」= 过 prepare；新增 `xylitol.obs.lane` / 直连降噪 req；scenarios 保持 `feature: false`
- [ ] 1.2 若需：`domain-compaction` 一句对齐「观测何时算实际执行」（避免与面事件混淆）
- [ ] 1.3 归档前：architecture「理想 vs 现状」勾选已兑现行；roadmap M-lane 段收缩

## 2. 发射：lane + prepare-first

- [ ] 2.1 统一 helper（或各 span start）为 llm 主路径写入 `xylitol.obs.lane=llm`
- [ ] 2.2 `CompactionOrchestrator::compact`：prepare-first；Ok 后 span+lane；Err 不建 `agent.compaction` OTLP
- [ ] 2.3 auto threshold/overflow：补 lane；确认仍 prepare-first
- [ ] 2.4 CollectingReporter 单测：早退无 compaction span；过闸有 span 且 lane=llm

## 3. 运维示例

- [ ] 3.1 添加 Collector 示例（按 `xylitol.obs.lane` 分流）+ 短注释/文档链
- [ ] 3.2 example / AGENTS 指针：默认仍直连 Langfuse；infra 时改 endpoint

## 4. 校验

- [ ] 4.1 `llman sdd validate c1870-add-otel-llm-infra-split --strict --no-check`
- [ ] 4.2 相关 lib 单测 + `just fmt` / clippy 触及面
