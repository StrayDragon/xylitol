# Design: 公开面死码清仓

> 依赖 c2700 的暂留落地（`#[allow(dead_code)]` 约 93 处 / 43 文件，2026-09-10 实测；
> c2700 时约 40 簇 / 1700 行——c2705/c2710/c2720/c2730/c2740 已消化其中一部分）。

## 方法（编译器驱动，不靠清单记忆）

1. 分批移除 `#[allow(dead_code)]`（整文件 allow 先行）→ `cargo check --all-features --all-targets` 收集 `never used` warning。
2. 逐 warning 分诊三态：
   - **真死** → 删 item + 其同文件单测 + 孤儿 re-export；
   - **误标（实际有引用，含 cfg(test)/serde/schema）** → 删 allow 本身，或改 `#[cfg(test)]`；
   - **有持久理由** → 保留 allow 但 MUST 注明理由（如 schema 字段供 LLM UX），没有注释理由的 allow 一律不接受。
3. 每批跑 `cargo test --lib --all-features`；全部完成后 `just qa`。

## 初始清单与分诊预判（apply 时以编译器为准）

- **整文件级 allow（4）**：
  - `agent/model/manifest.rs` — 零引用 → 整文件删（含 `mod` 声明）。
  - `infra/config/value.rs` — 零引用 → 整文件删。
  - `infra/process/child.rs` — 零引用 → 整文件删。
  - `agent/prompt/skill_expand.rs` — **有活项**（`expand_skills_in_agent_messages` re-export 在用）→ 逐项删死项、移除文件级 allow。
- **已标注 c2750 purge candidate**：`capabilities/mod.rs`（2 处）、`context_policy/mod.rs`（2 处，含 `STATUS_BAR_MODE_DEFAULT` 类默认值——删前确认无产品消费）、`infra/event/mod.rs` 第二总线方法面（`on`/`clear`/`on_lifecycle`/`UnsubscribeHandle`；`XyEventSink` 主路径保留）。
- **有注释理由、默认保留**：`infra/tools/bash.rs:35`（schema 字段供 LLM UX）；`bash.rs:252/259`（test-only seam → 评估改 `#[cfg(test)]`）。
- **其余簇**（clipboard osc52/native、settings manager/storage、trust store/resolve、provider factory、image resize、tools ask、protocol message/source_info/session helpers、model manager、capabilities stats/queue、tools freeze、mcp assemble、process shell、builder）：按方法逐项分诊；serde `#[serde(skip)]`/schema 字段类不允许整字段删除，除非确认不在 wire/LLM 契约。

## 分诊例外（跳过并记录）

后续 change（c2705/c2710/c2720/c2730/c2740）已激活或重写的簇——apply 时遇到 warning 不出现（删 allow 即编译干净）即属此类，直接删除 allow 即可，无需记录。

## 非目标

- 不新增行为、不改 wire、不动 specs（`skip_specs_landing: true`）。
- 不为「可能有用」保留任何无理由 allow（pre-0.0.1 卫生）。
