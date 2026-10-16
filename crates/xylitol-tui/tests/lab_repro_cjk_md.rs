//! lab 复现：62 列终端下 CJK + inline code 段落的 wrap 行为。

use xylitol_tui::{Component, Markdown, Palette, visible_width};

#[test]
fn lab_markdown_cjk_wrap_at_62() {
    let text = "我是你的编码助手，主要帮你在这个 `xylitol` 项目里干活。能力大致分几类：\n\n## 📖 读与查\n- **读代码/文档**：读任意文件（含图片），理解上下文\n- **搜索**：`grep` 全文正则检索、`find` 按 glob 找文件、`ls` 列目录\n- **LSP 诊断**：拿 rust-analyzer / gopls 等的报错、补全、符号跳转（项目里主要是 Rust）\n\n## ✏️ 改代码\n- 新建 / 覆盖 / 精确编辑文件\n- 按分层架构（`protocol/` → `agent/` → `infra/` → `app/`）改，遵守 `src/AGENTS.md` 边界\n\n## 🏃 跑验证\n- `just setup` / `fmt` / `lint` / `test` / `qa`（全量门禁）\n- 需要真终端的 e2e：`just qa-e2e`\n\n简单说：读得懂代码、改得了逻辑、跑得通门禁、跟得上 SDD 流程。你现在想做什么？比如「帮我看看 X 模块」「修个 bug」「新增 Y 功能」，或直接丢给我一个需求。";
    for width in [62usize, 63, 70, 71, 72] {
        let mut md = Markdown::new(
            text.to_string(),
            0,
            0,
            Palette::dark().markdown_theme(),
            None,
        );
        let lines = md.render(width);
        println!("== width {width} ==");
        for l in &lines {
            println!("[{:>3}] {}", visible_width(l), l);
        }
    }
}
