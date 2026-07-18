# Design: c1340

## write Tail

```text
… (N earlier lines, T total, ctrl+o to expand)
<last ≤10 lines of write_content>
```

Ctrl+O → full write_content（短文件，可接受）。

## 硬截断检测

`output` / bash body 含行前缀 `[Full output:` → `hard_truncated = true`。

## 渲染

| 块 | hard_truncated | Ctrl+O |
|---|---|---|
| write body | n/a | 可展开 |
| Tool output / Bash | true | `expanded` 强制 false；hint=`expand disabled — see Full output` |
| Tool output / Bash | false | 现有 Tail 5 + ctrl+o |

## End 替换

`truncated: true` 的 bash tool JSON → `UiEntry::Tool.output = combined`（已含 footer），丢弃 Update 累积的全量 chunk。
