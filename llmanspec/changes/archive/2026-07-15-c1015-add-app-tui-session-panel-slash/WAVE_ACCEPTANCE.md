# 会话 slash 迁移波次 — 人类验收与 E2E

覆盖归档：`c1005` → `c1010` → `c1015`。
刻意差异：`src/app/tui/PI_DELTAS.md`（A01–A06）。
计划修正：`../pi/_PLAN_REPORT.md`「调研修正」。

## 自动（已绿 / 复跑）

```bash
cargo test --lib h26 -- --test-threads=1   # session-tree + old unknown
cargo test --lib h27 -- --test-threads=1   # session-fork
cargo test --lib h28 -- --test-threads=1   # compact
cargo test --lib h29 -- --test-threads=1   # export
cargo test --lib h30 -- --test-threads=1   # import confirm
cargo test --lib h31 -- --test-threads=1   # /session dump
cargo test --lib h32 -- --test-threads=1   # /session-resume
cargo test --lib arch_guard -- --test-threads=1
# 可选真终端树（未钉旧 slash 名）：
# just test-tui-e2e-pty
```

各 change 明细：`ACCEPTANCE.md` / `VERIFY.md`（archive 目录内）。

## 人类手测清单（约 10 分钟）

前置：`cargo run -- --trust`（debug 构建更佳；可用 Fake）。

### 命名（c1005）

- [ ] `/` 补全含 `session-tree` / `session-fork`，无 `tree` / `fork`
- [ ] `/session-tree` 开树；`/tree` → unknown
- [ ] 有 leaf 时 `/session-fork` 切 child；`/fork` → unknown

### IO（c1010）

- [ ] `/session-compact` → 压缩提示（或未触发时的 did=false 提示）
- [ ] `/session-compact x` → usage 错误
- [ ] `/session-export` → 写出 HTML（默认），系统行含路径
- [ ] `/session-export /tmp/t.jsonl` → JSONL
- [ ] `/session-import /tmp/t.jsonl` → Yes/No 槽；No/Esc 取消；Yes 切换会话

### 面板（c1015）

- [ ] `/session` → scrollback 出现 Session Info / Messages 类统计（非操作菜单）
- [ ] `/session info` → 错误
- [ ] `/session-resume` → 列表槽；Esc 不切换；Enter 切换并刷新 transcript
- [ ] `/resume` → unknown

### 回归烟雾

- [ ] `/model` 仍开 picker；双 Esc 仍开树；`/exit` 退出

## 建议 commit（人类确认后）

```
feat(tui): session-* slash rename, io, and resume panel (c1005–c1015)
```
