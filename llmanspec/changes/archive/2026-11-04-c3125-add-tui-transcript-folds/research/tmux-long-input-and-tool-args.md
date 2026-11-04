# tmux 观测 — 长用户消息与工具参数

日期：2026-11-04；环境：macOS，tmux 3.8，debug/release `target/*/xylitol`，120×36。

## 复现装置

- 本地 Python `http.server` 实现 OpenAI Responses SSE；`xylitol serve` 启 host；`xylitol tui --trust --model local --port <p>` 在 tmux 中运行。
- 配置：`provider: openai` / `api: openai-responses` / `base_url` 指本地；`context_window: 128000`。
- 脚本：`/tmp/xyl_lab2.py`（thinking + 工具跳动）、`/tmp/xyl_lab4.py`（大工具参数）；未入库。
- 截屏：`/tmp/xyl_lab_artifacts2/`、`/tmp/xyl_lab_artifacts4/`。

## 长用户消息

- 编辑器粘贴 80 行：显示 `[paste #1 +80 lines]`，占 1 行；输入区高度不随内容增长。
- 提交后：用户消息全文 80 行进入 scrollback；屏幕 30 行全是用户文本；视口贴底，上一轮内容被挤出。
- 结论：编辑器 paste marker 只解决输入期；提交后需要用户消息折叠（c3125 设计 3 行 preview + 展开）。

## 工具参数

- 场景：bash 工具参数约 1k 字符（中文约 2k 列），展开簇后 header 换成 18 行，占满屏幕。
- 现有 header 用 `push_wrapped`，产品路径 `usize::MAX` 不截字符；没有 args 折叠层。
- 结论：header 必须单行截断；完整命令进 args viewport（c3125）。

## 帧序列（thinking 跳动）

- 6.74s：非空 24 行（thinking 正文流式）。
- 7.21s：非空 11 行（thinking flush 成 entry；簇默认收起）。
- 7.51s：非空 12 行（`Thought` 头 + 工具簇头 + 助手正文）。
- 结论：流式 thinking 需要一行 header（c3120），避免 N→1 的高度突变。
