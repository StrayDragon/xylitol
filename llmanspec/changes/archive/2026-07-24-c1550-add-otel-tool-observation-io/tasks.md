# Tasks: c1550-add-otel-tool-observation-io

## 1. 合约

- [x] 1.1 live `infra-otel`：otel14 req + feature 场景
- [x] 1.2 `llman sdd change attach c1550-add-otel-tool-observation-io`

## 2. 配置与闸

- [x] 2.1 `OtelConfig.tool_observation_io` + Default/解析测
- [x] 2.2 bridge `set_tool_observation_io_tier` / `tool_observation_io_tier`；`ObservationIoTier::max_chars` 公开
- [x] 2.3 `logging.rs` 组合根接线

## 3. 写入路径

- [x] 3.1 `ToolExecuteSpan::attach_io`；react 在结果落定后调用（含 deny）
- [x] 3.2 单测：none 不写；truncated 有上限属性

## 4. 校验

- [x] 4.1 `llman sdd validate c1550-add-otel-tool-observation-io --strict --no-check`
- [x] 4.2 相关单测 / `cargo test` 窄范围绿
