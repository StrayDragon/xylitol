# Design: c200-integrate-skills

## Skill Expansion Pipeline

```
User input: `/skill:rust-cli-tui-developer write a test`

1. Prompt detects `/skill:` prefix
2. Look up skill name in ResourceLoader.getSkills()
3. Read SKILL.md → extract body (strip frontmatter)
4. Generate XML block:
   `<skill name="rust-cli-tui-developer" location="/path/to/SKILL.md">\nReferences are relative to /path/to.\n\n{body}\n</skill>`
5. Append remaining args: `\n\nwrite a test`
6. Send expanded text to LLM
```

## System Prompt Skills Section

```xml
<available_skills>
  <skill>
    <name>rust-cli-tui-developer</name>
    <description>Use when building Rust CLI...</description>
    <location>/path/to/SKILL.md</location>
  </skill>
</available_skills>
```
