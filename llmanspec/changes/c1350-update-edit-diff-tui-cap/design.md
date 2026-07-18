# Design: c1350

## 根因

```text
matched_old_texts[0]  ──diff──►  final_content (整文件)
generate_display_diff: iter_all_changes 含全部 Equal
TUI: render_diff_lines + word_level 对上万行
```

## 生成

- `generate_unified_diff` / `generate_display_diff(old_file, new_file)`：整文件。
- display：基于 `unified_diff().context_radius(3)` 转 gutter；另 `MAX_DISPLAY_DIFF_LINES=200`，超出追加 `(diff truncated for display, N lines omitted)`。

## TUI

- `MAX_DIFF_RENDER_LINES=80`；超出保留前缀行 + warning 脚注。
- `lines > 120` 或字符超阈值 → `word_level: false`。
