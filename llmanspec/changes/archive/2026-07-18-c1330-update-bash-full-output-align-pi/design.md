# Design: c1330 Full output align pi

## Footer 格式（SSOT）

```text
[Full output: <abs-or-tmp-path>. Truncated: <N> lines shown (<limit> limit)]
```

- `<limit>`：与 `DEFAULT_MAX_BYTES` 一致的人类可读尺寸（如 `50.0KB`）。
- 无 path（spill 失败）时 path 写 `(unavailable)`，仍 MUST 标 Truncated。
- 未截断：不追加 footer。

## 数据流

```text
OutputAccumulator.finish()
  → display_content() = tail + optional footer
  → XyBashResult.output / tool JSON "combined"|"stdout"(截断)
  → history / project_for_llm（bang）
  → TUI scrollback（warning paint on footer line）
```

## 非流式 bash 工具 JSON

截断或未截断均：

| 字段 | 内容 |
|------|------|
| `stdout` / `combined` | **仅** `display_content()`（合并流的截断视图） |
| `stderr` | `""`（已并入 combined；禁止再塞全量 stderr） |
| `full_output_path` | Option |
| `truncated` | bool |

## TUI

检测输出行 `starts_with("[Full output:")` → `theme` warning fg（对齐 DESIGN `colors.warning`）。不新增 UiEntry 字段（路径已在正文内，便于进 context）。
