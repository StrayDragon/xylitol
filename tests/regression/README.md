# Regression tests

Drop one file per regression under this directory using the naming pattern:

`{issue_number}-{short-description}.rs`

These files are not compiled automatically by Cargo unless they are included by a top-level
integration test. They exist as a curated template + reference archive for adding new regressions.
