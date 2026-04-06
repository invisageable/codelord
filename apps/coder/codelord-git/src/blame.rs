//! Git blame functionality.
//!
//! Parses `git blame --incremental` output for efficient line-by-line blame.

use rustc_hash::FxHashMap as HashMap;

use std::path::Path;
use std::process::Command;

/// A single blame entry for a range of lines.
#[derive(Debug, Clone)]
pub struct BlameEntry {
  /// Commit SHA (40 hex chars).
  pub sha: String,
  /// Author name.
  pub author: String,
  /// Author email.
  pub author_email: String,
  /// Unix timestamp of the commit.
  pub timestamp: i64,
  /// Commit summary (first line of message).
  pub summary: String,
  /// First line number (1-indexed).
  pub start_line: usize,
  /// Last line number (1-indexed, inclusive).
  pub end_line: usize,
}

/// Commit metadata shared across blame entries.
#[derive(Debug, Clone)]
pub struct CommitInfo {
  /// Commit SHA.
  pub sha: String,
  /// Author name.
  pub author: String,
  /// Commit summary.
  pub summary: String,
  /// Unix timestamp.
  pub timestamp: i64,
}

/// Parsed blame result for a file.
#[derive(Debug, Default, Clone)]
pub struct Blame {
  /// Blame entries sorted by start_line.
  pub entries: Vec<BlameEntry>,
  /// Commit info indexed by SHA.
  pub commits: HashMap<String, CommitInfo>,
}

impl Blame {
  /// Find blame entry for a specific line (1-indexed).
  pub fn entry_for_line(&self, line: usize) -> Option<&BlameEntry> {
    self
      .entries
      .iter()
      .find(|e| line >= e.start_line && line <= e.end_line)
  }

  /// Get commit info by SHA.
  pub fn commit(&self, sha: &str) -> Option<&CommitInfo> {
    self.commits.get(sha)
  }

  /// Check if blame data is empty.
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  /// Total number of blame entries.
  pub fn len(&self) -> usize {
    self.entries.len()
  }
}

/// Run git blame on a file and parse the result.
///
/// # Arguments
/// * `repo_path` - Path to the git repository root.
/// * `file_path` - Path to the file (relative to repo or absolute).
///
/// # Returns
/// `Some(Blame)` on success, `None` if git command fails.
pub fn blame_file(repo_path: &Path, file_path: &Path) -> Option<Blame> {
  let output = Command::new("git")
    .args([
      "blame",
      "--incremental",
      "-w", // ignore whitespace changes
      "--",
    ])
    .arg(file_path)
    .current_dir(repo_path)
    .output()
    .ok()?;

  if !output.status.success() {
    // Expected for new files not yet in git - silently return None.
    return None;
  }

  parse_incremental_blame(&String::from_utf8_lossy(&output.stdout))
}

/// Run git blame on file contents (not yet committed).
///
/// Uses `--contents -` to read from stdin.
pub fn blame_contents(
  repo_path: &Path,
  file_path: &Path,
  contents: &str,
) -> Option<Blame> {
  use std::io::Write;
  use std::process::Stdio;

  let mut child = Command::new("git")
    .args(["blame", "--incremental", "-w", "--contents", "-", "--"])
    .arg(file_path)
    .current_dir(repo_path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .ok()?;

  if let Some(mut stdin) = child.stdin.take() {
    let _ = stdin.write_all(contents.as_bytes());
  }

  let output = child.wait_with_output().ok()?;

  if !output.status.success() {
    return None;
  }

  parse_incremental_blame(&String::from_utf8_lossy(&output.stdout))
}

/// Parse git blame --incremental output.
fn parse_incremental_blame(output: &str) -> Option<Blame> {
  let mut blame = Blame::default();
  let mut lines = output.lines().peekable();

  while let Some(line) = lines.next() {
    // Header format: <sha> <orig_line> <final_line> <num_lines>
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 4 {
      continue;
    }

    // SHA must be 40 hex characters
    let sha = parts[0];
    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
      continue;
    }

    let start_line: usize = match parts[2].parse() {
      Ok(n) => n,
      Err(_) => continue,
    };

    let num_lines: usize = match parts[3].parse() {
      Ok(n) => n,
      Err(_) => continue,
    };

    let mut author = String::new();
    let mut author_email = String::new();
    let mut timestamp = 0i64;
    let mut summary = String::new();

    // Parse metadata lines until we hit content or next header
    while let Some(&next) = lines.peek() {
      // Content line starts with tab
      if next.starts_with('\t') {
        lines.next();
        break;
      }

      // Next header line (40-char hex)
      if next.len() >= 40
        && next.chars().take(40).all(|c| c.is_ascii_hexdigit())
      {
        break;
      }

      let next = lines.next().unwrap();

      if let Some(val) = next.strip_prefix("author ") {
        author = val.to_string();
      } else if let Some(val) = next.strip_prefix("author-mail ") {
        author_email = val.trim_matches(&['<', '>', ' '][..]).to_string();
      } else if let Some(val) = next.strip_prefix("author-time ") {
        timestamp = val.parse().unwrap_or(0);
      } else if let Some(val) = next.strip_prefix("summary ") {
        summary = val.to_string();
      }
      // Skip: author-tz, committer*, previous, filename, boundary
    }

    let sha = sha.to_string();

    // git blame --incremental only outputs metadata once per commit.
    // If we've seen this SHA before, reuse cached metadata.
    if let Some(cached) = blame.commits.get(&sha) {
      if author.is_empty() {
        author = cached.author.clone();
      }
      if timestamp == 0 {
        timestamp = cached.timestamp;
      }
      if summary.is_empty() {
        summary = cached.summary.clone();
      }
    }

    blame.entries.push(BlameEntry {
      sha: sha.clone(),
      author: author.clone(),
      author_email,
      timestamp,
      summary: summary.clone(),
      start_line,
      end_line: start_line + num_lines - 1,
    });

    // Cache commit info for reuse.
    blame.commits.entry(sha.clone()).or_insert(CommitInfo {
      sha,
      author,
      summary,
      timestamp,
    });
  }

  // Sort entries by start line
  blame.entries.sort_by_key(|e| e.start_line);

  Some(blame)
}

/// Format timestamp as relative time (e.g., "2 days ago").
pub fn relative_time(timestamp: i64) -> String {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);

  let diff = now - timestamp;

  if diff < 0 {
    return "in the future".to_string();
  }

  let seconds = diff;
  let minutes = seconds / 60;
  let hours = minutes / 60;
  let days = hours / 24;
  let weeks = days / 7;
  let months = days / 30;
  let years = days / 365;

  if years > 0 {
    format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
  } else if months > 0 {
    format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
  } else if weeks > 0 {
    format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
  } else if days > 0 {
    format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
  } else if hours > 0 {
    format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
  } else if minutes > 0 {
    format!("{} min{} ago", minutes, if minutes == 1 { "" } else { "s" })
  } else {
    "just now".to_string()
  }
}

/// Find the git repository root for a given path.
///
/// Walks up the directory tree looking for a `.git` directory.
pub fn find_repo_root(path: &Path) -> Option<std::path::PathBuf> {
  let output = Command::new("git")
    .args(["rev-parse", "--show-toplevel"])
    .current_dir(path.parent()?)
    .output()
    .ok()?;

  if !output.status.success() {
    return None;
  }

  let root = String::from_utf8_lossy(&output.stdout).trim().to_string();

  if root.is_empty() {
    None
  } else {
    Some(std::path::PathBuf::from(root))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_relative_time() {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs() as i64;

    assert_eq!(relative_time(now), "just now");
    assert_eq!(relative_time(now - 120), "2 mins ago");
    assert_eq!(relative_time(now - 3600), "1 hour ago");
    assert_eq!(relative_time(now - 86400), "1 day ago");
  }
}
