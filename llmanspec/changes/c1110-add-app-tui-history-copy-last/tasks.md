# Tasks — c1110-add-app-tui-history-copy-last

## 1. Seam + slash

- [x] 1.1 `Driver::copy_text_to_clipboard` + InProcess / Scripted
- [x] 1.2 `commands` / catalog：`/history-copy-last`；busy 允许
- [x] 1.3 `effects`：最后 assistant → copy → 系统块

## 2. 测试

- [x] 2.1 harness：有 assistant 时 Scripted 记录 copy；无则提示
- [x] 2.2（可选）usage / catalog

## 3. 校验

- [x] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1110-… --no-interactive`
- [x] 3.2 `just lint` + 相关测
