# Future — c520 Extension 逃生舱

本变更**不实现**。记录可选后续，避免把厂商私有事件抬进 `XyEvent` 闭集。

## `XyEvent::Extension`（可选）

```text
XyEvent::Extension { namespace: String, payload: serde_json::Value }
```

- 用途：极少数跨面实验 / 调试载荷；**不是**主路径，也不是厂商事件宽表。
- 应用面默认：ignore + tracing，不 panic。
- 触发条件：现有 `XyChunk` / Message 载荷仍表达不了、且确实需要跨面消费时再议。
- 禁止：用 Extension 绕过「先改 adapter → 再扩 Chunk」的正常路径。

## 相关

- 两层模型 SSOT：同目录 `design.md`
- Queue 语义实现：`c525-add-async-concurrent-queue`
