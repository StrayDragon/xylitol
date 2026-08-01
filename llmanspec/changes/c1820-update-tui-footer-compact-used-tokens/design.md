# Design: compact used tokens (c1820)

## Decision

**Reuse** `format_compact_tokens` for the used count inside `footer_token_label`.

| Range | Form |
|---|---|
| `< 1_000` | decimal digits |
| `< 10_000` | one decimal `k` (`1.0k`…`9.9k`) |
| `< 1_000_000` | rounded `Nk` |
| else | `M` with same thresholds |

Provenance:

| Provenance | Shape |
|---|---|
| Api / RemoteCount / LocalTokenizer | `used {compact} tokens` |
| Heuristic | `used ~{compact} tokens` |
| Unknown | `used ? tokens` (no compact) |

Derived `%/W` unchanged (`atc21`); `W` already compact.

## Non-goals

- Second formatter for used only
- Dropping the word `tokens`
- Changing compaction scrollback thousands separators
