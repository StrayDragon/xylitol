# Design — c1125-add-app-tui-at-path-completion

## 接线

`UiRoot::install_completion_sources` 在 slash / model arg 源之后注册：

```text
SlashArg(/debug?) → SlashArg(/model) → SlashCommandSource → AtPathSource(at_path_base)
```

`at_path_base` 默认 `std::env::current_dir()`；harness 经 `set_at_path_base` 注入 tempdir。

Footer 展示用的 `cwd` 字符串（可含 `~`）**不**作为 AtPath 根，避免把 display 路径当成真实目录。

## 提交语义

选定 `@path` 后 editor 中为路径引用文本；本变更 **不** 在 submit 时读文件进消息。与 demo/pi 路径引用对齐。

## 明确不做

- `$` skill source
- 跨盘任意路径默认扫描
- 改 package AtPath 模糊算法
