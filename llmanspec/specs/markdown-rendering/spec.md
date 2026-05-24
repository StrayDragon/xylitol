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
requirements[2]{req_id,title,statement}:
  r1,"terminal-markdown-rendering",System MUST render Markdown into terminal styled lines with stable wrapping for wide characters and code blocks.
  r2,"terminal-markdown-links",System MUST avoid forcing link underline and link color when rendering Markdown unless explicitly enabled by configuration.
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,TUI output is enabled,assistant content contains Markdown with code blocks and CJK characters,rendered lines wrap within viewport width and preserve styling
  r2,happy,Markdown contains links,renderer uses default configuration,rendered output does not apply underline and blue color unconditionally
```
