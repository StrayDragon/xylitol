---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c90-update-markdown-rendering"
---

```toon
kind: llman.sdd.spec
name: "markdown-rendering"
purpose: "TBD - created by archiving change c90-update-markdown-rendering. Update purpose after archive."
requirements[6]{req_id,title,statement}:
  r1,"leaf-core-rendering","System MUST delegate Markdown rendering to leaf-core library while preserving existing pub(crate) interface contract."
  r2,"terminal-markdown-links",System MUST avoid forcing link underline and link color when rendering Markdown unless explicitly enabled by configuration.
  r3,"table-layout","System MUST render Markdown tables with aligned columns and border decorations via leaf-core table layout engine."
  r4,"theme-system","System MUST support configurable rendering themes via leaf-core theme system including built-in presets."
  r5,"github-alert-callouts","System MUST render GitHub-style alert callouts (Note/Tip/Warning/Caution) with distinct visual styling."
  r6,"ratatui-version-boundary",System MUST handle ratatui version differences at the MarkdownRenderer boundary without exposing version conflicts to callers.
scenarios[6]{req_id,id,given,when,then}:
  r1,"leaf-delegate",MarkdownRenderer is instantiated,render method is called with Markdown content,"rendering is delegated to leaf_core::MarkdownRenderer and output matches leaf-core styled lines"
  r2,happy,Markdown contains links,renderer uses default configuration,rendered output does not apply underline and blue color unconditionally
  r3,"table-render",Markdown input contains a GFM table,renderer processes the content,output displays table with aligned columns and visible borders
  r4,"theme-switch","a non-default theme is configured",Markdown content is rendered,output reflects the configured theme colors and decorations
  r5,"alert-render",Markdown input contains a GitHub Alert block,renderer processes the content,output displays the alert with appropriate icon and styled border
  r6,"version-compat","leaf-core re-exports ratatui 0.30 types",MarkdownRenderer returns styled lines to xylitol TUI,returned types are compatible with xylitol ratatui 0.29 rendering pipeline
```
