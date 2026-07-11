# 架构图与业务流

> **只写高维产品/业务思考与流程图。**
> 具体类型、API、文件路径、crate 选型 → 只出现在 `llmanspec/changes/*/design.md` 与代码。
> 规范边界：根 / `src` `AGENTS.md`。短索引：`_NOTE.md`。

| 文档 | 内容 |
|---|---|
| [overview.md](./overview.md) | 产品怎么分层、什么开箱、什么后置 |
| [library-and-clients.md](./library-and-clients.md) | 库嵌入 + 多 client 矩阵；理想 vs 现状 |
| [turn-flow.md](./turn-flow.md) | 用户一轮对话怎么走 |
| [provider.md](./provider.md) | 多厂商模型如何对用户保持一致 |
| [queue-and-interrupt.md](./queue-and-interrupt.md) | 插话 / 续跑 / 中止的产品语义 |
| [events.md](./events.md) | 用户可见事件 vs 厂商细节 |

实现与变更：`llmanspec/changes/`（嵌入 API、Server-on-Driver、线协议对齐；历史：c500–c525 已归档）。
