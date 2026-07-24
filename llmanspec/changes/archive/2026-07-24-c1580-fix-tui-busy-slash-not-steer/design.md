# c1580 Design

## Seam

产品 TUI harness：`HostSession` busy + Enter 提交 slash → 断言 `take_steer` 空 + Allow 走 Driver / Reject 系统提示。

## Policy 表

```text
busy_slash_policy(PendingSlash) -> Allow | Reject   # exhaustive match
try_busy_input:
  Some(slash) -> clear editor; policy Allow ? pending.slash|quit : refuse note
  None + looks_like_unknown_slash -> refuse note; MUST NOT steer
  else bang reject / steer (unchanged)
```

effects：Allow 命令（SessionName/Compact/Export/SessionDump/…）去掉 busy 二次拒绝。

## 开闭

- 新 `PendingSlash` 变体强制进 `busy_slash_policy` match（编译穷尽）
- `try_busy_input` 不再散落 if 特判
