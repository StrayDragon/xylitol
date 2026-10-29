# Reverted (2026-10-29)

c2995 was reverted from `main`: built-in presets only covered a few compatible endpoints, duplicated connection details in the kernel, and drifted easily. The direction moved to supplier plugins / gateway integration discussed in #35.

- Revert PR: #36
- Discussion: #35
- Original issue: #20 (closed as not planned)

This directory stays as design history; the code, specs, schema, example config and docs for presets were removed by the revert PR.
