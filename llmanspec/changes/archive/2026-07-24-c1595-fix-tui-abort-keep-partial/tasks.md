# c1595 Tasks

## 1. TUI flush partial

- [x] 1.1 abort latch：flush streaming → entries + abort 脚注；再 clear 缓冲
- [x] 1.2 修订 `note_user_abort` / lifecycle `Error("aborted")`：不抹掉已 flush 的 Assistant
- [x] 1.3 harness：mid-stream Esc 保留上行正文 + 脚注；迟到 delta 仍不追加
- [x] 1.4 test: `cargo test -p xylitol --lib -- c665_busy_esc harness_busy_ctrl_c abort`

## 2. ReAct persist + LLM filter

- [x] 2.1 cancel 路径 persist partial assistant（`stop_reason: Aborted`）；空 content 可跳过
- [x] 2.2 `project_for_llm` 跳过 aborted/error assistant
- [x] 2.3 单测：投影过滤；可选 react abort persist
- [x] 2.4 test: `cargo test -p xylitol --lib -- project_for_llm abort`

## 3. 合约与文档

- [x] 3.1 新增/修订 ati / ar requirement + `.feature` 场景
- [x] 3.2 同步 `插话续跑与中止.md` + `PI_DELTAS`（登记曾偏离、现对齐）
- [x] 3.3 attach + validate --strict
