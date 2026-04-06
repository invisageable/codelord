Here's a pragmatic git blame module for zo:
```rust
  // ide-core/src/git/blame.rs

  use std::collections::HashMap;
  use std::path::Path;
  use std::process::Command;

  /// A single blame entry for a range of lines.
  #[derive(Debug, Clone)]
  pub struct BlameEntry {
    pub sha: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
    pub summary: String,
    pub start_line: usize,
    pub end_line: usize,
  }

  /// Parsed blame result for a file.
  #[derive(Debug, Default)]
  pub struct Blame {
    pub entries: Vec<BlameEntry>,
    pub commits: HashMap<String, CommitInfo>,
  }

  #[derive(Debug, Clone)]
  pub struct CommitInfo {
    pub sha: String,
    pub author: String,
    pub summary: String,
    pub timestamp: i64,
  }

  /// Run git blame on a file.
  pub fn blame_file(repo_path: &Path, file_path: &Path) -> Option<Blame> {
    let output = Command::new("git")
      .args([
        "blame",
        "--incremental",
        "-w", // ignore whitespace
        "--",
      ])
      .arg(file_path)
      .current_dir(repo_path)
      .output()
      .ok()?;

    if !output.status.success() {
      return None;
    }

    parse_incremental_blame(&String::from_utf8_lossy(&output.stdout))
  }

  fn parse_incremental_blame(output: &str) -> Option<Blame> {
    let mut blame = Blame::default();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
      // Format: <sha> <orig_line> <final_line> <num_lines>
      let parts: Vec<&str> = line.split_whitespace().collect();
      if parts.len() < 4 || parts[0].len() != 40 {
        continue;
      }

      let sha = parts[0].to_string();
      let start_line: usize = parts[2].parse().ok()?;
      let num_lines: usize = parts[3].parse().ok()?;

      let mut author = String::new();
      let mut author_email = String::new();
      let mut timestamp = 0i64;
      let mut summary = String::new();

      // Parse metadata lines until next sha or filename
      while let Some(&next) = lines.peek() {
        if next.starts_with('\t') || next.chars().next().map(|c|
  c.is_ascii_hexdigit()).unwrap_or(false) && next.len() >= 40 {
          break;
        }
        let next = lines.next().unwrap();

        if let Some(val) = next.strip_prefix("author ") {
          author = val.to_string();
        } else if let Some(val) = next.strip_prefix("author-mail ") {
          author_email = val.trim_matches(&['<', '>'][..]).to_string();
        } else if let Some(val) = next.strip_prefix("author-time ") {
          timestamp = val.parse().unwrap_or(0);
        } else if let Some(val) = next.strip_prefix("summary ") {
          summary = val.to_string();
        }
      }

      // Skip content line (starts with \t)
      if lines.peek().map(|l| l.starts_with('\t')).unwrap_or(false) {
        lines.next();
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

      blame.commits.entry(sha.clone()).or_insert(CommitInfo {
        sha,
        author,
        summary,
        timestamp,
      });
    }

    Some(blame)
  }

  Usage in editor:
  // Query blame for visible lines
  fn blame_for_row(&self, row: usize) -> Option<&BlameEntry> {
    self.entries.iter().find(|e| row >= e.start_line && row <= e.end_line)
  }
  ```