//! Git repository discovery.
//!
//! Walks up from cwd to find `.git` directory or worktree file.

use std::path::{Path, PathBuf};

/// Paths discovered for a git repository.
#[derive(Debug, Clone)]
pub struct GitPaths {
    /// Repository root directory (where `.git` lives).
    pub repo_dir: PathBuf,
    /// Common git directory (for worktrees, this resolves commondir).
    pub common_git_dir: PathBuf,
    /// Path to HEAD file.
    pub head_path: PathBuf,
}

/// Find git metadata paths by walking up from `cwd`.
pub fn find_git_repo(cwd: &Path) -> Option<GitPaths> {
    let mut dir = Some(cwd.to_path_buf());

    while let Some(current) = dir {
        let git_path = current.join(".git");
        if git_path.exists() {
            if git_path.is_file() {
                // Worktree: .git is a file containing "gitdir: <path>"
                let content = std::fs::read_to_string(&git_path).ok()?;
                let git_dir_str = content.strip_prefix("gitdir: ")?.trim();
                let git_dir = current.join(git_dir_str);
                let head_path = git_dir.join("HEAD");
                if !head_path.exists() {
                    return None;
                }
                let common_dir_path = git_dir.join("commondir");
                let common_git_dir = if common_dir_path.exists() {
                    let common = std::fs::read_to_string(&common_dir_path).ok()?;
                    let common = common.trim();
                    if Path::new(common).is_absolute() {
                        PathBuf::from(common)
                    } else {
                        git_dir.join(common)
                    }
                } else {
                    git_dir.clone()
                };
                return Some(GitPaths {
                    repo_dir: current,
                    common_git_dir,
                    head_path,
                });
            } else if git_path.is_dir() {
                // Standard repo
                let head_path = git_path.join("HEAD");
                if !head_path.exists() {
                    return None;
                }
                return Some(GitPaths {
                    repo_dir: current,
                    common_git_dir: git_path,
                    head_path,
                });
            }
        }
        let parent = current.parent()?.to_path_buf();
        if parent == current {
            return None;
        }
        dir = Some(parent);
    }

    None
}
