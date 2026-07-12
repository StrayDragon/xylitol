# design — c485 vertical slice

## 问题

功能已接通，缺「一轮可聊」的可重复证明：组件 harness 碎片绿；`tests/tui_e2e` 只打 `agent_demo`。

## 验收真相源（硬约束）

```text
合成 harness A+B  →  H1–H10（合约主路径）
产品 PTY smoke    →  MUST（Fake 默认一句 + /exit）
人工真 TTY        →  verify 清单一行（可选记录）
```

## A. 合成 harness

### 层

| 层 | 内容 | c485 |
|---|---|---|
| A. UI 合成 | `HostSession` + `HostEvent::Xy` | MUST |
| B. 假 Driver 合流 | `ScriptedDriver` + 薄编排（抽自 `run_host_loop` 消费 pending/流） | MUST |
| C. 全环 `select!` 注入 | 真 `run_host_loop` | 不做 |

### ScriptedDriver

- `run(prompt)` → 记下 prompt，返回预置 `XyEvent` 流
- `abort` / `steer` / `follow_up` / `clear_queue` / `queue_stats` → 记调用
- `current_model` → 稳定假模型，供 footer

### H1–H10

| ID | Then |
|---|---|
| H1 | idle Enter → `Driver::run`；Busy |
| H2 | TextDelta→AgentEnd → scrollback + Idle |
| H3 | Tool* → 可见 `UiEntry::Tool` |
| H4 | busy Enter → `steer` + `Steering:` chrome，无 `[steer]` 墙 |
| H5 | Alt+Enter → `follow_up` + `Follow-up:` |
| H6 | Alt+Up → 还原 editor + `clear_queue(true,true)` |
| H7 | Esc abort 后再 idle 提交第二次 `run` 成功（ati14） |
| H8 | `/exit` → quit + `finish_inline`（TestTerminal 可观测 stop） |
| H9 | `/model` pending/dispatch 轻断言 |
| H10 | preflight 非 TTY / NoModel（已有则挂清单） |

## B. 产品 PTY smoke（MUST）

| 项 | 决议 |
|---|---|
| 二进制 | 产品 `xylitol`（非 `agent_demo`） |
| 配置 | temp dir：`XyModelKind::Fake` + 选中模型；`--trust` 跳过 Ask |
| Ready needle | footer 含模型名或 cwd 点分（实现时钉一条稳定串） |
| 一轮 | 提交任意短 prompt → 屏上出现 **`Hello from fake provider`**（Fake 默认文案 SSOT） |
| 退出 | 输入 `/exit` → 进程结束（exit 0） |
| 不做 | steer/abort/工具 PTY；跨进程 `set_fake_text`；Trust Ask 全键 |

**限制**：Fake 脚本化是 thread-local，子进程不可用——PTY 只依赖默认文案；多轮/工具仍归合成 harness。

## C. 非目标

c492 / c493 / 活树 / c575 / c470 / 为 PTY 新建 Fake 文件协议。
