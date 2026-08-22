# Design

## 渲染规则

```
{marker} {title} {loc} (timeout {N}s)  (Alt+E)
                              ^^^^^^^^^ muted key_hint（既有）
              ^^^^^^^^^^^^^^^ muted，仅 Some(N) 时
```

- 词形：`(timeout {N}s)`，N 为钳制后的生效值；与 `Compacted from N tokens (Alt+E to expand)` 的「信息 + 按键提示」节奏一致。
- 仅 bash/grep/find 有模型面；fs 工具无参数故永不显示。

## 数据流

```
XyEvent args JSON ──► upsert_tool_entry 提取 timeout(i64>0 → u64)
                  ──► UiEntry::Tool.timeout_secs
                  ──► cache hash 字段 ──► paint_tool_header_line(Some/None)
```

session_tree 重建（resume）首批不带（args 可得处后续票补），显示缺口可接受。

## 设计稿 / 词表

- `designing/tui/modules/expandable/states/tool-timeout.yaml`：collapsed 行含 `(timeout 600s)  (Alt+E)`。
- intent.md 第 5 点旁注规则追加 timeout 段说明。
- chrome 词汇表对话条目节登记固定词。

## 测试

- paint 单测：Some(600) → header 含 `(timeout 600s)` 且位于 `(Alt+E)` 前；None → 不含。
- bridge 单测：upsert 带 args.timeout → entry 字段生效；无 timeout → None。
