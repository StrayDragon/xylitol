# 02 TUI 与 server 的能力归属（切分观察，非裁决）

> 拆开之后问的是：哪段代码还在当前终端进程里跑。能力按自然归属可分两半：**终端里看见 / 按到 / 粘贴的**属本机面，**改仓库 / 调模型 / 跑 MCP / 跑工作区 shell** 属工作区面。归属裁决本身（P6 等）在提案；本篇只给切分观察与现状纠缠点。
> 对照：`XyDriver` 在 `src/app/core/driver/proto.rs`——进程内实现把很多能力塞进同一个 Driver，拆开时本就要大改。

## 能力归属两分

属本机终端面（自然贴在你坐的那台机器 + 这个 TTY）：

| 能力 | 干什么 | 为什么是本机面 |
|---|---|---|
| `copy_text_to_clipboard` | 复制一段历史 | 剪贴板是你坐的那台机器 |
| `read_clipboard_text` | Ctrl+V | 同上 |
| `stage_clipboard_image` | 粘贴图片 | 同上；OSC 52 也得写进这个终端 |
| 键位、paint、TTY、`$EDITOR` 改输入框 | 面本地 | 没工作区什么事 |

属工作区 / 仓库面（操作器的一部分）：`run` / `abort` / 模型 / 会话 / compact / steer / 队列 / MCP / 工具 / **工作区 bash（含 `!`）** / `persist_project_trust` 落盘。`/trust` 是人在 TUI 里点的，写入的是 **server 那个项目** 的信任文件。

reload 的归属：键位与主题属本机面；skills / MCP / prompt 属工作区面。

## 完整交互路径（prompt：人打字 → 屏幕多几个字）

1. 输入框敲 `fix the bug`，回车。TUI 还没调模型，发一条 `Command::Prompt`。
2. Server 收下开始一轮；模型每吐一点，推一条 `Event::TextDelta`（如 `"Hel"` 再 `"lo"`）。
3. TUI 只把字追加到画面，不知道模型是什么 API。
4. 模型要改文件：server 在工作区跑工具，推 `ToolStart` / `ToolEnd`，TUI 画工具摘要。
5. 模型卡住要你选：server 做反向 RPC，TUI 弹选项；你选完发 `ApproveTool` / `AnswerQuestion`，server 继续。

中间是 tagged JSON 的线协议；换编码只换字节格式，步骤不变。

## 灰色地带（按「路径」还是「工作区」切）

| 事情 | 切分观察 |
|---|---|
| `export_html` / `export_jsonl` | 你填的是本机路径 → server 给会话内容，TUI 写盘；要写进工作区另说 |
| `import_jsonl` | 文件在你电脑上 → TUI 读完发给 server |
| `session_store()` 把 `Arc` 塞给 TUI | 进程内实现的历史行为；Remote 实现已返回 `None` |

## 搬迁时会大的现状纠缠点（与 HTTP 框架无关）

- Driver 上剪贴板三方法；
- bang 与 `execute_bash` 缠在一起（见 `03`：人 `!` 与模型工具是两种语义，现在挤在同一个 Driver 方法里）；
- export 路径当本机文件还是工作区文件。

这些是「产品切分」的工作量，不是换个框架能蒙混过去的。
