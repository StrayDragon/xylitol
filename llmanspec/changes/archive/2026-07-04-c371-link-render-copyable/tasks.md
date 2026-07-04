# c371-link-render-copyable — Tasks

> 只改 inline.rs 一个文件。链接色复用 primary_color（零侵入）。

## 阶段 1：修复链接渲染

- [x] 1.1 `inline.rs:225-235`：`[text](url)` 渲染改为 `text (url)` 多 Span。变量 `_url`→`url`。text 用 text_color，括号用 muted_text_color，url 用 primary_color + UNDERLINED。空 link_text 只输出 url。
- [x] 1.2 `inline.rs:198` 后：加 `<url>` autolink 分支（扫 `<` 到 `>`，校验 `://`/`mailto:`，渲染 url）。

## 阶段 2：测试 + 校验

- [x] 2.1 新增 TestBackend 测试：`[text](url)` → text + url 都可见；`<url>` → url 可见无尖括号；`[](url)` → 只 url。
- [x] 2.2 回归现有 12 个 markdown 测试 + 全 lib 测试。
- [x] 2.3 `just qa`（all-features）+ `arch_guard` 4 + `llman sdd validate --strict` 通过。
- [x] 2.4 归档 c371。

## 反降级护栏

- [x] `[text](url)` 渲染后 url 完整可见（可手动复制，spec tui72）。
- [x] `<url>` 渲染为干净 url（无尖括号）。
