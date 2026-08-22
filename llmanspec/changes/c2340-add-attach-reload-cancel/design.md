# Design — attach reload 合作取消

## 数据流（改后）

```
TUI reload 态 Esc → input_policy request_reload_cancel() → cancel token 触发
  └─ Remote reload_runtime select!:
       biased; _ = cancel.cancelled() => {
           drop(reload_unary);          // 释放 &mut 借用
           let _ = self.unary("abort")  // Host: abort_reload() 合作取消
           return report { cancelled: true, steps: [] }
       }
       data = &mut reload_unary => 解析 steps/cancelled（原路径不变）
```

Host `abort` unary 现行语义（落约为 sr-abort1）：命中进程级 reload → cancel + 回 `{cancelled:true}`；未命中 → 落回会话 abort 处理。reload 仅 idle 可发起，与 run abort 无并发冲突。

## 关键点

- **借用**：select 内先 `drop(fut)` 再发 abort，避免同时两个 `&mut self` unary 借用。
- **abort 尽力而为**：其响应被忽略（网络失败不掩盖取消意图）；本地报告直接以 cancelled 收尾，UI 不等 Host 二次确认。
- **原路径零改动**：正常完成分支解析逻辑保持逐字段一致。

## 测试边界

`HostClient` trait mock（remote 测试内既有 SnapClient 注入先例）：
- mock `unary("reload")` 挂起直至 abort 到达；
- 驱动：起 reload_runtime → 取消 token → 断言 abort 被调用且返回 `cancelled=true`。
- 反向对照：不取消时走原解析路径（既有行为，现有覆盖即可）。
