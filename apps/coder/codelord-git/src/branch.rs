//! Git branch detection.

use std::path::Path;

/// Detect current git branch for a workspace.
///
/// Reads `.git/HEAD` to determine the current branch or commit.
/// Returns `None` if not a git repository.
pub fn detect_branch(workspace_path: &Path) -> Option<String> {
  let git_head = workspace_path.join(".git/HEAD");

  std::fs::read_to_string(&git_head).ok().and_then(|content| {
    let content = content.trim();

    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
      // Normal branch reference
      Some(branch.to_string())
    } else if !content.is_empty() {
      // Detached HEAD - return short commit hash
      Some(content.chars().take(7).collect())
    } else {
      None
    }
  })
}

/// Check if workspace is in detached HEAD state.
pub fn is_detached_head(workspace_path: &Path) -> bool {
  let git_head = workspace_path.join(".git/HEAD");

  std::fs::read_to_string(&git_head)
    .map(|c| !c.trim().starts_with("ref:"))
    .unwrap_or(false)
}

/// Check if workspace has uncommitted changes.
///
/// Runs `git status --porcelain` and returns true if output is non-empty.
pub fn check_git_dirty(workspace_path: &Path) -> bool {
  std::process::Command::new("git")
    .args(["status", "--porcelain"])
    .current_dir(workspace_path)
    .output()
    .map(|o| !o.stdout.is_empty())
    .unwrap_or(false)
}
