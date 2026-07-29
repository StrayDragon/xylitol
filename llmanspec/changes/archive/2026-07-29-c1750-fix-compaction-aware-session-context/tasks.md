# Tasks: c1750-fix-compaction-aware-session-context

## 1. Specs

- [x] 1.1 收紧 as45 / as47 / ar32：history 播种与 overflow retry MUST 经 leaf + compaction-aware firstKept 裁切
- [x] 1.2 package-ai-bridge pab18：Responses 错误体整型 code 时 MUST 暴露上游 message

## 2. Tests first

- [x] 2.1 单测：path 含 compaction 时 context 不含 firstKept 之前的消息
- [x] 2.2 单测：extract 从 content JSON 抽出 exceed_context message

## 3. Implementation

- [x] 3.1 `build_context_entries` + 接线 build_session_context(_v2)
- [x] 3.2 `load_conversation_history` → leaf + 裁切
- [x] 3.3 overflow will_retry 重载 history
- [x] 3.4 Responses map_err 抽文案
- [x] 3.5 estimate_from_session_entries 同源裁切

## 4. Gate

- [x] 4.1 validate + 相关 cargo test
