# c1560 Design

## Seam

产品 TUI harness：`HostSession` + Fake/`ScriptedDriver`（可注入 `list_sessions` / `load_entries`）→ 断言 editor ↑ 召回。配置：`AppConfig` 单测。

## 算法

```text
seed_editor_history(mode):
  texts = []
  if mode == New:
    sessions = list_sessions()
      .filter(cwd_matches(current_cwd))
      .filter(id != current_id)
      .sort_by(mtime desc)
      .take(N)
    for s in sessions.rev():          # 旧 → 新
      texts.extend(user_texts(s))     # 会话内时间序；跳过 trim 后以 / 开头
  else: # ResumeOrSwitch
    texts = user_texts(current_id)
  editor.clear_history(); for t in texts { add_to_history(t) }
```

`user_texts`：从 `load_entries` 抽 user 角色纯文本（与现有 message 投影一致；多 part 则拼接或取文本 part——实现时跟 session entry 形状，禁止塞 tool/assistant）。

## 开闭

- N 与 cwd 过滤只经配置 + 既有 store API；不新 port
- Host 调 Driver/store 只读；不 reach `agent::` 内部

## 风险

- 大 session 启动 IO：N 默认 1；可后续加上限条数（本 change 不做硬顶除非测到痛点）
