---
depends_on: []
---

# c105-add-export-capabilities: 会话导出（HTML / JSONL / import / share）

## Why
pi 支持 export-html（ANSI→HTML + tool-renderer）、`exportToJsonl()`、`import` JSONL、`share`（GitHub gist）。xylitol 完全缺失，会话无法归档、分享、迁移或导入。这是 RPC 模式（c115）`export_html` / `get_messages` 命令和 slash `/export` `/import` `/share` 的前置依赖。

## What Changes
- 新增 `src/infra/session/export/{html,jsonl,import}.rs`：
  - `export_html(session) -> String`：消息/工具结果转可读 HTML（基础 ANSI→HTML 转义，tool 块代码化渲染）
  - `export_jsonl(session) -> Vec<u8>`：逐行 JSON 对象
  - `import_jsonl(bytes) -> Vec<SessionEntry>`：版本校验 + entry 反序列化
- `AgentSession` 增 `export_to_html(path)` / `export_to_jsonl(path)` / `import_from_jsonl(path) -> new_session_id`
- `share_as_gist()`：接口 stub，未配置 token 时返回明确的配置引导消息（不强制实现网络上传）

## Capabilities
- session-persistence

## Impact
- 非破坏性：新增模块 + AgentSession 方法。
- 无新依赖（HTML 用手写转义；如需更丰富渲染可后续引入，本期不做）。
- 触及文件：`src/infra/session/export/`（新增）、`src/infra/session/mod.rs`（导出）、`src/agent/session.rs`（方法）。
