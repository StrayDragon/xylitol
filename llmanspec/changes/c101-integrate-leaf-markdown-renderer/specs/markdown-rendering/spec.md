```toon
kind: llman.sdd.delta
ops[5]{op,req_id,title,statement,from,to,name}:
  modify_requirement,r1,"leaf-core-rendering","System MUST delegate Markdown rendering to leaf-core library while preserving existing pub(crate) interface contract.",null,null,null
  add_requirement,r3,"table-layout","System MUST render Markdown tables with aligned columns and border decorations via leaf-core table layout engine.",null,null,null
  add_requirement,r4,"theme-system","System MUST support configurable rendering themes via leaf-core theme system including built-in presets.",null,null,null
  add_requirement,r5,"github-alert-callouts","System MUST render GitHub-style alert callouts (Note/Tip/Warning/Caution) with distinct visual styling.",null,null,null
  add_requirement,r6,"ratatui-version-boundary","System MUST handle ratatui version differences at the MarkdownRenderer boundary without exposing version conflicts to callers.",null,null,null
op_scenarios[5]{req_id,id,given,when,then}:
  r1,leaf-delegate,"MarkdownRenderer is instantiated","render method is called with Markdown content","rendering is delegated to leaf_core::MarkdownRenderer and output matches leaf-core styled lines"
  r3,table-render,"Markdown input contains a GFM table","renderer processes the content","output displays table with aligned columns and visible borders"
  r4,theme-switch,"a non-default theme is configured","Markdown content is rendered","output reflects the configured theme colors and decorations"
  r5,alert-render,"Markdown input contains a GitHub Alert block","renderer processes the content","output displays the alert with appropriate icon and styled border"
  r6,version-compat,"leaf-core re-exports ratatui 0.30 types","MarkdownRenderer returns styled lines to xylitol TUI","returned types are compatible with xylitol ratatui 0.29 rendering pipeline"
```
