# c35-add-repeat-detection Tasks

- [x] 实现 RepeatDetector（滑动窗口 + n-gram HashSet）
- [x] 实现检测参数配置（min_n, max_n, window_size, thresholds）
- [x] 实现中断逻辑（cancel_generation）
- [x] 实现 RecoveryManager（alter_prompt / switch_model / adjust_params / delegate_to_planner）
- [x] 实现 recovery 配置解析
- [x] 作为模型输出流中间件集成到 agent loop
- [x] 编写测试：检测准确性、恢复策略链
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c35-add-repeat-detection --strict --no-interactive`
