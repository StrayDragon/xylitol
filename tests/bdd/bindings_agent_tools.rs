use crate::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-entire"
)]
fn test_read_entire(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-offset-limit"
)]
fn test_read_offset_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-missing"
)]
fn test_read_missing(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-offset-oob"
)]
fn test_read_offset_oob(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-truncate"
)]
fn test_read_truncate(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "read-missing-path"
)]
fn test_read_missing_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-new"
)]
fn test_write_new(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-parents"
)]
fn test_write_parents(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-overwrite"
)]
fn test_write_overwrite(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-byte-count"
)]
fn test_write_byte_count(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-missing-path"
)]
fn test_write_missing_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "write-missing-content"
)]
fn test_write_missing_content(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-single"
)]
fn test_edit_single(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-multi"
)]
fn test_edit_multi(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-overlap"
)]
fn test_edit_overlap(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-nonunique"
)]
fn test_edit_nonunique(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-empty-old"
)]
fn test_edit_empty_old(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-noop"
)]
fn test_edit_noop(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-crlf"
)]
fn test_edit_crlf(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-bom"
)]
fn test_edit_bom(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-unicode"
)]
fn test_edit_unicode(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "edit-diff"
)]
fn test_edit_diff(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-echo"
)]
fn test_bash_echo(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-stderr"
)]
fn test_bash_stderr(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-exit-code"
)]
fn test_bash_exit_code(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-timeout"
)]
fn test_bash_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-merged-streams"
)]
fn test_bash_merged_streams(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-truncate"
)]
fn test_bash_truncate(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-result-no-full-dump"
)]
fn test_bash_result_no_full_dump(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-cancel"
)]
fn test_bash_cancel(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-missing-cmd"
)]
fn test_bash_missing_cmd(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-basic"
)]
fn test_grep_basic(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-no-match"
)]
fn test_grep_no_match(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-limit"
)]
fn test_grep_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-ignore-case"
)]
fn test_grep_ignore_case(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-literal"
)]
fn test_grep_literal(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "grep-missing-pattern"
)]
fn test_grep_missing_pattern(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-simple"
)]
fn test_find_simple(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-recursive"
)]
fn test_find_recursive(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-limit"
)]
fn test_find_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-no-match"
)]
fn test_find_no_match(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-bad-path"
)]
fn test_find_bad_path(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "find-absolute"
)]
fn test_find_absolute(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-empty"
)]
fn test_ls_empty(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-entries"
)]
fn test_ls_entries(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-sorted"
)]
fn test_ls_sorted(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-default-cwd"
)]
fn test_ls_default_cwd(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-limit"
)]
fn test_ls_limit(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-missing"
)]
fn test_ls_missing(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "ls-not-dir"
)]
fn test_ls_not_dir(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "large-file"
)]
fn test_tools_large_file(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "output-overflow"
)]
fn test_tools_output_overflow(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "negative-timeout"
)]
fn test_tools_neg_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "zero-timeout-rejected"
)]
fn test_tools_zero_timeout(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "absolute-pattern"
)]
fn test_tools_abs_pattern(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "multibyte-truncation"
)]
fn test_tools_multibyte(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "fuzzy"
)]
fn test_tools_fuzzy(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "span"
)]
fn test_tools_span(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "small-output"
)]
fn test_tools_small_output(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "overflow-temp-file"
)]
fn test_tools_overflow_temp(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-uses-accumulator"
)]
fn test_tools_bash_accum(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "png-yields-image-part"
)]
fn test_tools_png_img(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "text-still-works"
)]
fn test_tools_text_works(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-omit-timeout-completes"
)]
fn test_tools_bash_omit_timeout(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "all-ten-tools-smoke"
)]
fn test_tools_all_ten_smoke(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "fudiff-line-offset"
)]
fn test_tools_fudiff_line_offset(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "require-str-missing-arg"
)]
fn test_tools_require_str_missing(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "error-mapping"
)]
fn test_tools_error_mapping(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "cancel"
)]
fn test_tools_cancel(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "registry"
)]
fn test_tools_registry(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "infra-works"
)]
fn test_tools_infra_works(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "toolset-unit-ops"
)]
fn test_tools_toolset_ops(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "tool-write-in-session-workspace"
)]
fn test_tools_write_in_session_workspace(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-tools/agent-tools.feature",
    name = "bash-runs-in-session-workspace"
)]
fn test_tools_bash_runs_in_session_workspace(ws: Workspace) {}
