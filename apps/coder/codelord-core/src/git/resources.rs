use codelord_git::Blame;

use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;

use std::path::PathBuf;

/// Global git blame settings.
#[derive(Resource, Debug, Clone)]
pub struct GitBlameSettings {
  /// Whether git blame is enabled by default for new tabs.
  pub enabled: bool,
  /// Minimum column position for inline blame text.
  pub min_column: u32,
}

impl Default for GitBlameSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      min_column: 60,
    }
  }
}

/// Result from a background blame operation.
pub struct BlameResult {
  pub entity: Entity,
  pub blame: Option<Blame>,
}

/// Resource for pending async blame operations.
#[derive(Resource)]
pub struct PendingBlameRequests {
  pub receiver: flume::Receiver<BlameResult>,
  pub sender: flume::Sender<BlameResult>,
}

impl Default for PendingBlameRequests {
  fn default() -> Self {
    let (sender, receiver) = flume::unbounded();

    Self { sender, receiver }
  }
}

/// Git branch and status tracking for the active workspace.
#[derive(Resource, Debug, Default, Clone)]
pub struct GitBranchState {
  /// Current branch name (or short commit hash if detached).
  pub branch: Option<String>,
  /// Whether we're in detached HEAD state.
  pub is_detached: bool,
  /// Whether the working directory has uncommitted changes.
  pub is_dirty: bool,
  /// Workspace path this state belongs to (to detect changes).
  pub workspace_path: Option<PathBuf>,
  /// Whether a branch detection is currently in progress.
  pub loading: bool,
}

/// Result from background branch detection.
pub struct BranchResult {
  pub workspace_path: PathBuf,
  pub branch: Option<String>,
  pub is_detached: bool,
}

/// Result from background dirty status check.
pub struct StatusResult {
  pub workspace_path: PathBuf,
  pub is_dirty: bool,
}

/// Resource for pending async branch detection.
#[derive(Resource)]
pub struct PendingBranchRequests {
  pub branch_receiver: flume::Receiver<BranchResult>,
  pub branch_sender: flume::Sender<BranchResult>,
  pub status_receiver: flume::Receiver<StatusResult>,
  pub status_sender: flume::Sender<StatusResult>,
}

impl Default for PendingBranchRequests {
  fn default() -> Self {
    let (branch_sender, branch_receiver) = flume::unbounded();
    let (status_sender, status_receiver) = flume::unbounded();

    Self {
      branch_sender,
      branch_receiver,
      status_sender,
      status_receiver,
    }
  }
}
