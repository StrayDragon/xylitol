---
depends_on: [c25-add-agent-loop]
---

# c35-add-repeat-detection

## Why

本地小模型（7B/13B）容易陷入 token 循环，远程模型在惩罚参数不足时同样可能重复。需要在软件层建立与模型无关的重复检测与自动恢复机制（§10）。

## What Changes

1. 在 `src/agent/` 实现流式重复检测中间件（`RepeatDetector`）
2. 滑动窗口 + n-gram 集合（HashSet）进行实时检测
3. 中断推理机制（`cancel_generation()`）
4. 恢复管理器（`RecoveryManager`）：alter_prompt / switch_model / adjust_params / delegate_to_planner
5. 配置驱动（`repeat_detection` YAML 段）

### 检测算法

- 维护滑动窗口（W 个 token）+ n-gram 集合（min_n..max_n）
- 连续命中超过阈值或窗口内重复占比超阈值 → 判定循环
- 立即中断推理，触发恢复策略

### 恢复策略链

```
alter_prompt（加反重复提示）→ switch_model（切换到另一 provider）→ adjust_params（调高惩罚）→ delegate_to_planner
```

## Capabilities

- `repeat-detection`: 流式 n-gram 重复检测 + 中断 + 可配置恢复策略

## Impact

- 新增 `ahash` 或使用 `std::collections::HashSet`
- 作为模型输出流的中间件，对上层透明
- 触发 `repeat_detected` hook 事件（与 c40 联动）
- **始终编译**：无 feature flag，通过 `repeat_detection.enabled` config 控制运行时启用
