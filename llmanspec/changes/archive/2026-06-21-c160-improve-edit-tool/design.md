# c160-improve-edit-tool: Design

## Changes to `src/agent/tools/patch.rs`

### 1. `normalize_for_fuzzy_match(text) -> String`
- NFKC normalize
- Smart quotes → ASCII: `\u2018\u2019` → `'`, `\u201c\u201d` → `"`
- Dashes → ASCII hyphen: en-dash, em-dash, figure dash, minus → `-`
- Unicode spaces → regular space: NBSP, various width spaces → ` `
- Strip trailing whitespace per line

### 2. `detect_line_ending(content) -> LineEnding`
- Return `CRLF` or `LF` based on first `\r\n` occurrence

### 3. `restore_line_endings(text, ending) -> String`
- Normalize to LF internally, then replace with CRLF if needed

### 4. `find_span(content, old_text) -> Option<Span>`
- Split old_text into line segments
- Try exact match first, then progressively loosen with fuzzy matching

### 5. Diff output
- Use `similar::TextDiff` to compute diff of before/after for review

## Dependencies

- Already has `similar` in Cargo.toml
- No new dependencies
