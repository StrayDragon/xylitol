# Tasks — 会话名全量套件竞态

> 实施者导读：design §0 嫌疑链与「未解之谜」诚实框；修复选根因级改法。收口走 finalize。

- [ ] T1 首日复核嫌疑链：读 log/{store,project}.rs 对应行确认 with_id=false 与 map.insert 丢弃；若有出入更新 design §0
- [ ] T2 修复：append_session_entry with_id=true + 投影无 id 行防御收纳
- [ ] T3 钉 bug 的单测（set→get 立即读回）+ 既有测试绿
- [ ] T4 验收：全量 just test 连续 5 次零失败（记录 5 次运行摘要于 research）
