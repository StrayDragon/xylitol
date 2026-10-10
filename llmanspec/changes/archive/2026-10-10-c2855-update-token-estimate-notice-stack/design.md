# 设计：Api 优先启发式与客户端通知栈

## 计量

```text
有可信 Done.usage 锚点 → Api（usage + trailing 启发式）
否则                         → Heuristic（整段启发式）
```

启发式：对投影后消息 `serde_json::to_string` 的 UTF-8 字节 `div_ceil(HEURISTIC_BYTES_PER_TOKEN)`，`HEURISTIC_BYTES_PER_TOKEN = 3`。切点逐条切片、fixed_context 开销、摘要占位、图片 `ESTIMATED_IMAGE_CHARS` 折算 MUST 引用同一常数。lax 反序列化失败仍用该除数，不得另开 `/4`。

RemoteCount HTTP（Anthropic `count_tokens` / OpenAI `input_tokens`）与 LocalTokenizer encode **退出产品路径**；实现删除。再引入条件见 xylitol-dev-candidates skill。

## 配置与 CLI

删除：顶层 `tokenizers:`、`token_estimate.local_tokenizer`、`models.*.tokenizer`、`xylitol tokenizer *`。根 `AppConfig` 无 `deny_unknown_fields`，旧键忽略。**不**加加载 warn / resources doctor 提示。

## 客户端通知栈（TUI 端侧，非 Host 操作器）

两槽分工：

| 槽 | 谁用 | 形态 |
|---|---|---|
| **通知栈**（新） | 成功/说明：Copied、首次 Heuristic 提示、既有 Info toast | 右上角浮层，最新在上，最多 3，TTL 自动消失；图标成功 `✓`、说明 `◆`；不进 ScrollNotice、不计入下缘 footprint |
| **通知条**（既有 atc22） | 拒闸/失败 | status 上方 1 行，`Error: ` + warning |

Copied 从 status 旁独立 cue 迁入通知栈。首次 Heuristic：仅当整段 `provenance == Heuristic`（不是 Api trailing）时，每个 TUI 进程推一次；Print 不提示。

本波不做：栈内 Tab/回车动作、展开侧板、落盘「终身一次」。

晋级：`designing/tui-lab/modules/toast-stack/` → `designing/tui/modules/toast-stack/`，intent 与词汇表同步；键盘键从 lab 草稿降为后波。

## 明确不做

- 旧 YAML 告警或自动改写用户文件
- 为启发式引入第二套百分比闸
- 复活 LocalTokenizer / 静默下载词表
- 跨端 gpui 通知栈（端未开放；同源词条先写进词汇表）
