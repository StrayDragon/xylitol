# c1570 Design

## Seam

产品 TUI harness：`HostSession` + `HostEvent::Input(Ctrl+C)`；busy 经 `run_active` / `UiPhase::Busy` / `bash_active`。

## 分流

```text
HostSession::step(Input):
  try_busy_input first:
    busy && !overlay && app.clear (Ctrl+C):
      same latch as app.interrupt (Esc) → pending.abort
      MUST NOT quit
  else dispatch → UiRoot::on_ctrl_c:
    overlay → close_slot
    non-empty editor → clear
    empty → quit_flag
```

Busy 路径抢在 listener 之前，避免空 editor 误 quit。

## 开闭

- 不新增键位 id；复用 `app.clear` / `app.interrupt`
- 不改 Esc 语义
