# Tasks

- [x] 1. Implement spec requirements:
  - `calculate_context_tokens(usage)` — priority total_tokens > sum (c1)
  - `estimate_context_tokens(messages, last_usage)` — chars/4 fallback (c3)
  - `should_compact(context_tokens, context_window, settings)` — threshold check (s2)
- [x] 2. Write 5 unit tests (calculate_context_tokens total/sum, should_compact trigger/disabled, estimate_context_tokens)
- [x] 3. `cargo test` 21 compaction + 5 BDD = 26 compaction tests PASS; 367 total PASS
- [x] 4. Run `llman sdd validate c50-align-compaction --strict --no-interactive`
- [x] 5. Run `just qa` ✅
