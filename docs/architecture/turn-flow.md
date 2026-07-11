# 一轮对话（产品时序）

> 用户视角：从输入到结束发生了什么。

```mermaid
sequenceDiagram
  actor U as 用户
  participant UI as 界面
  participant Agent as 对话核心
  participant Model as 模型
  participant Tools as 工具
  participant Mem as 会话记忆

  U->>UI: 输入问题（或插话 / 中止）
  UI->>Agent: 开始或改道本轮
  Agent->>Mem: 带上已有上下文
  loop 直到本轮结束
    Agent->>Model: 请继续
    Model-->>UI: 文字 / 思考 / 要调用工具
    alt 模型要动手
      Agent->>Tools: 执行（可一批多个）
      Tools-->>UI: 进度与结果
      Agent->>Mem: 记下结果
    else 模型说完且无后续插话
      Agent-->>UI: 本轮结束
    end
  end
```

## 插话与中止（产品语义）

详见 [queue-and-interrupt.md](./queue-and-interrupt.md)。

- **插话（steer）**：本轮还在跑时补充指令，下一拍再听，不粗暴掐断正在做的工具（除非用户中止）。
- **续跑（follow-up）**：本轮全部结束后再开一问；中止时仍可把未发出的续跑文案还给输入框。
- **中止（abort）**：停掉进行中的工作；清掉未消化的插话；保留未发出的续跑以便恢复。
