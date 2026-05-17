---
depends_on: [c10-add-config, c25-add-agent-loop, c55-add-planning-execution]
---

# c60-add-model-lock（Phase 2 — 延后实施）

> **⚠️ Phase 2**: MVP 仅使用远程 API（OpenAI-compatible + Anthropic-compatible），无需模型抢占锁。本 change 延后至支持本地模型（GGUF/llama.cpp 等串行执行引擎）时实施。

## Why

本地 GGUF 模型串行执行，需要对每个模型实例实施抢占锁，支持优先级排队和检查点恢复，防止低优先级任务长时间占用（§6）。

## What Changes

1. `ModelRegistry`：模型实例生命周期管理、Provider 注册
2. `LockManager`：锁状态机（Idle → Busy → Preemptible → Preempting）、等待队列
3. `CheckpointManager`：保存/恢复被中断任务的上下文
4. 基于 priority + deadline 的抢占决策

### 锁状态机

```
Idle → Busy（任务获得锁）
Busy → Preemptible（高优先级到达）
Preemptible → Preempting（保存检查点）
Preempting → Idle（锁转交高优任务）
Busy → Idle（任务完成释放锁）
```

### 组件

- `SchedulerHint`：任务声明预期资源占用
- 长任务可被打断，短任务不可抢占

## Capabilities

- `model-lock`: 模型抢占锁 + ModelRegistry + LockManager + CheckpointManager

## Impact

- `src/agent/model.rs` 从占位变为实际实现
- feature flag `agent-model-lock` 启用此模块
- 依赖 c55 规划-执行分离的调度器
