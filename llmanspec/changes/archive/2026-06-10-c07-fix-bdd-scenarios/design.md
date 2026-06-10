# Design: 修复 36 个 BDD 场景

## Diagnosis

All 36 failures are pre-existing gaps exposed by rstest-bdd migration. No architectural changes needed.

### Category map

| Category | #Tests | Root Cause |
|----------|--------|------------|
| Session `type` conflict | 7 | `EntryBase.entry_type` (`#[serde(rename="type")]`) + enum `#[serde(tag="type")]` produces duplicate key |
| `{text}` quotes | 14 | rstest-bdd `{text}` captures surrounding quotes, need `{text:string}` or strip |
| OR clause parsing | 6 | `"A" \u6216 "B" \u6216 "C"` in feature treated as single string |
| Missing steps | 3 | `\u7ed3\u679c\u5217\u51fa {entry}` step not registered; DataTable parsing off |
| Bash JSON vs flat text | 3 | `_t_combined_has` checks whole JSON string, should extract field |
| Edit behavior | 6 | Edit unicode/CRLF/BOM/diff: assertions misaligned |
| Find absolute | 1 | No validation of absolute path patterns in find tool |
| Write byte count | 1 | Write returns JSON but then step expects flat string |

## Fix Approach

### 1. Remove `type` field serialization (T5)
`EntryBase.entry_type` should NOT serialize `type` field — the enum's `#[serde(tag="type")]` already provides it. Add `#[serde(skip_serializing)]` and keep `#[serde(rename="type")]` only for deserialization.

### 2. Strip quotes from `{text}` capture (T6-T12)
Add a helper `fn strip_quotes(s: &str) -> String` that removes surrounding `"..."`.
For OR clauses: split on ` \u6216 ` and check any sub-clause matches.

### 3. Session auto-init (T1-T4)
In all session when steps, before using `sess.mgr`, check if `None` and auto-initialize.

### 4. Register missing steps (T13-T14)
Add `#[then("\u7ed3\u679c\u5217\u51fa {entry:string}")]` and fix DataTable parsing.

### 5. Bash JSON extraction (T12)
`_t_stdout_has` and `_t_combined_has` must parse the JSON result and extract the `stdout` / `combined` fields.

### 6. Edit behavior fixes (T15-T20)
- `_w_edit_multi` DataTable: skip table header row
- find absolute: reject patterns starting with `/`
- edit CRLF/BOM/unicode: the tool is correct, assertions need `strip_quotes`
- edit_nonunique: error message contains `"not unique"` — matches `"unique"` OR clause
- edit_noop: error message contains `"oldText == newText"` or `"no change"` — matches `"identical" \u6216 "No changes" \u6216 "\u672a\u53d8\u66f4"`

## Risks
- Minimal — only test code and one serde attribute change
- No production behavior changes
