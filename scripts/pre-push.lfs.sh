#!/bin/sh
# Git LFS pre-push — installed as `.git/hooks/pre-push.legacy` by `just setup`.
#
# prek's configured pre-push hooks cannot feed git's pre-push stdin to
# `git-lfs pre-push` (prek closes hook stdin), so we use prek's legacy-hook
# mechanism instead: prek runs this after its own pre-push hooks and
# forwards git's stdin (the push lines) to it.
#
# The `pre-push.legacy` file lives outside git tracking; reinstall with:
#   just setup
command -v git-lfs >/dev/null 2>&1 || {
  echo >&2 "\nThis repository is configured for Git LFS but 'git-lfs' was not found on your path."
  echo >&2 "Install Git LFS: https://git-lfs.com/  (or skip LFS upload with GIT_LFS_SKIP_SMUDGE=1)."
  exit 2
}

git lfs pre-push "$@"
