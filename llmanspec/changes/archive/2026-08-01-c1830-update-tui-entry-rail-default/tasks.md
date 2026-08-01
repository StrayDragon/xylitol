# Tasks

## 1. Specs landing（合约）

- [x] 1.1 Branch binding：`llman sdd change start c1830-update-tui-entry-rail-default`（须干净树；否则先提交/stash 调研与 demo）
- [x] 1.2 改写 `app-tui-transcript`：att4/5/9/10/11/14（及依赖场景）→ rail 默认；更新 `app-tui-transcript.feature`
- [x] 1.3 改写 `app-tui-chrome` atc8 + feature（取消默认 user-message-bg）
- [x] 1.4 扩展 `package-tui-theme`：rail paint helper req；修正/确认 `valid_scope` 含 `packages/xylitol-tui/`
- [x] 1.5 `llman sdd validate` specs + commit Specs landing

## 2. 包原语

- [x] 2.1 实现并导出 `paint_left_rail_line`（轨 1 + gutter 1 + fit content；窄宽安全）[blocked-by: 1.4]
- [x] 2.2 包单测覆盖宽/窄与 `\x1b[49m` 复位约定 [blocked-by: 2.1]

## 3. 产品 scrollback

- [x] 3.1 `widgets/scrollback.rs`：Tool/Diff/Bash/Thinking 改 rail；去掉默认整行 tool-*-bg / padding_y 空 tint [blocked-by: 2.1]
- [x] 3.2 User：去掉默认 user-message-bg；保留语义前缀 [blocked-by: 3.1]
- [x] 3.3 Diff：解绑洗底信封；保持无 diff-*-bg 行底 [blocked-by: 3.1]
- [x] 3.4 更新产品单测与 BDD 场景断言（轨存在、洗底缺席、块间隙）[blocked-by: 3.1, 1.2]

## 4. 设计文与对照

- [x] 4.1 同步 `design/transcript.md` / `expandable.md` / 必要 `DESIGN.md` 段落为 rail 默认
- [x] 4.2 `agent_demo` 对齐包 helper（可选保留 wash 对照）；roadmap `TUI重制.md` 标明 M1 兑现中 [blocked-by: 2.1]

## 5. 门禁

- [x] 5.1 `just test-tui` + 相关 app-tui BDD；`just qa`（或至少 lint/test 相关切片）[blocked-by: 3.4, 4.1]
