# c1565 Design

## Seam

CLI 公共边界：`CliArgs` clap 解析（单测）+ 退出后 stderr resume 行（`maybe_print_resume_hint` 经 `XyDriver::session_id` + `list_sessions`）。

## Flag 归属

```text
root: 无 --session/--model/--list-models/--trust/--no-trust/--config/--no-color
tui [surface…] [run [surface…]]: 全部表面旗标；parent 与 run 合并（任一侧 set 即生效）
print [surface…] [PROMPT]: --session/--model/--config/--no-color
```

## Resume 提示

```text
after tui.finish / print.run success:
  sid = driver.session_id()?
  if list_sessions().any(id == sid):
    eprintln!("Resume by $ xylitol tui --session {sid}")
```

未进 `list_sessions`（未持久化 / 列表失败）→ 不打印。

## 开闭

- 不改 bootstrap 语义，仅改 CLI 传参入口
- 不 reach `agent::`；检测经既有 Driver seam
