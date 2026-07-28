# Tasks: c1660-add-compaction-overflow-retry

> 验收一致：overflow → 一次 compact-and-retry；与 threshold/manual 分流；不越界 c1670/c1680。

## 1. 合约

- [x] 1.1 修订 live `domain-compaction`：overflow 识别、一次 recovery、`reason=overflow`、与 retry 互斥；修订 `agent-runtime`：turn-end Case1 先于 threshold、willRetry 续跑；`.feature` 锚点四则
- [x] 1.2 design 编排流 / 事件字段已定稿（本 change `design.md`）

## 2. 识别与事件

- [ ] 2.1 实现 `is_context_overflow_assistant`（usage / length+零输出 / 模式集 + non-overflow 排除）；单测钉模式与排除
  `[blocked-by: 1.1]`
- [ ] 2.2 `retry` 路径改用同源检测，overflow MUST NOT AutoRetry
  `[blocked-by: 2.1]`
- [ ] 2.3 扩展 `CompactionEnd`（及映射）：`reason` / `will_retry` / `error_message`；Start reason 支持 `overflow`
  `[blocked-by: 1.2]`

## 3. 编排

- [ ] 3.1 turn-end：Case1 overflow（sameModel + stale + once flag + 摘错 assistant + willRetry）先于 Case2 threshold
  `[blocked-by: 2.1]` `[blocked-by: 2.3]`
- [ ] 3.2 二次 overflow：固定失败文案，不再 compact/retry；新 user prompt 复位 attempt 标志
  `[blocked-by: 3.1]`

## 4. 验收

- [ ] 4.1 单测/BDD：overflow-retry-ok、overflow-once、wrong-model、reason-overflow
  `[blocked-by: 3.2]`
- [ ] 4.2 `llman sdd validate c1660-add-compaction-overflow-retry --strict`；确认无 instructions/TUI%/可配置 max-retries
  `[blocked-by: 4.1]`
