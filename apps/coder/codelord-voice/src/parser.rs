//! Voice intent parser — translates text to VoiceActions.
//!
//! Converts transcribed speech into VoiceAction when server is unavailable.

use codelord_protocol::voice::model::VoiceAction;

use regex::Regex;

/// Voice intent parser.
///
/// Parses transcribed text and converts it to VoiceActions.
#[derive(Clone)]
pub struct VoiceIntentParser {
  command_patterns: Vec<CommandPattern>,
}

impl VoiceIntentParser {
  /// Creates a new parser.
  pub fn new() -> Self {
    let command_patterns = Self::build_patterns();

    Self { command_patterns }
  }

  /// Parse transcribed text into a VoiceAction.
  ///
  /// Returns Some(VoiceAction) if a command was recognized,
  /// None if no pattern matched.
  pub fn parse(&self, text: &str) -> Option<VoiceAction> {
    let text = text.to_lowercase();

    // Clean up the text (remove punctuation, extra whitespace).
    let command = text
      .trim()
      .trim_start_matches(&[',', '.', '!', '?'][..])
      .trim()
      .trim_end_matches(&[',', '.', '!', '?'][..])
      .trim();

    log::debug!("Parsing command: '{command}'");

    // Match against command patterns.
    for pattern in &self.command_patterns {
      if pattern.regex.is_match(command) {
        let action = VoiceAction {
          action: pattern.action_name.clone(),
          payload: None,
        };

        log::info!("Voice command matched: '{text}' -> {:?}", action.action);

        return Some(action);
      }
    }

    None
  }

  /// Build the default command patterns.
  /// Uses same format as server (PascalCase action names).
  fn build_patterns() -> Vec<CommandPattern> {
    let patterns = [
      // File operations.
      (r"(open|load)\s+(a\s+|the\s+)?file", "OpenFile"),
      (r"(open|load)\s+(a\s+|the\s+)?folder", "OpenFolder"),
      (r"save(\s+file)?", "SaveActiveTab"),
      // Tab operations.
      (r"(new|create)\s+tab", "NewTab"),
      (r"close(\s+the)?\s+tab", "CloseActiveTab"),
      (r"close\s+all(\s+the)?\s+tabs", "CloseAllTabs"),
      (r"close(\s+the)?\s+other\s+tabs", "CloseOtherTabs"),
      (
        r"close(\s+all)?(\s+the)?\s+tabs(\s+to)?(\s+the)?\s+right",
        "CloseTabsToRight",
      ),
      (r"(prev|previous)\s+tab", "PrevTab"),
      (r"next\s+tab", "NextTab"),
      // Search operations.
      (r"toggle\s+search", "ToggleSearch"),
      (r"(show|open)\s+(the\s+)?search", "ShowSearch"),
      (r"(hide|close)\s+(the\s+)?search", "HideSearch"),
      (r"find\s+next", "FindNext"),
      (r"find\s+previous", "FindPrevious"),
      // Panel toggles.
      (r"(toggle|show|hide)\s+explorer", "ToggleExplorer"),
      (r"(toggle|show|hide)\s+terminal", "ToggleTerminal"),
      (r"(toggle|show|hide)\s+copilord", "ToggleCopilord"),
      (r"(toggle|show|hide)\s+player", "TogglePlayer"),
      // Compiler stages.
      (
        r"run\s+(tokenizer|tokens|lexical)\s*(phase|stage|analysis)?",
        "RunTokensStage",
      ),
      (
        r"run\s+(parser|tree|syntax)\s*(phase|stage|analysis)?",
        "RunTreeStage",
      ),
      (
        r"run\s+(semantic|sir)\s*(phase|stage|analysis)?",
        "RunSirStage",
      ),
      (
        r"run\s+(codegen|asm|assembly)\s*(phase|stage)?",
        "RunAsmStage",
      ),
      (r"run\s+ui\s*(phase|stage)?", "RunUiStage"),
      (r"run\s+(benchmarks?|bench)", "RunBenchmarks"),
      (r"run\s+(the\s+)?tests?", "RunTests"),
      // Window operations.
      (r"center\s+(the\s+)?(window|editor)", "CenterWindow"),
      (r"shake\s+(the\s+)?(window|editor)", "ShakeWindow"),
      (r"vibrate\s+(the\s+)?(window|editor)", "ShakeWindow"),
      (
        r"(position|move|snap)\s+(window\s+)?(to\s+)?(the\s+)?left",
        "PositionWindowLeftHalf",
      ),
      (
        r"(position|move|snap)\s+(window\s+)?(to\s+)?(the\s+)?right",
        "PositionWindowRightHalf",
      ),
      // Music player.
      (
        r"(play|pause)\s+(the\s+)?(music|song|track)?",
        "PlayPauseMusic",
      ),
      (r"(show|open)\s+(the\s+)?playlist", "ShowPlaylist"),
      (r"(hide|close)\s+(the\s+)?playlist", "HidePlaylist"),
      // Focus mode.
      (r"(toggle\s+)?focus\s+mode", "ToggleFocusModeEditor"),
      // Hidden files.
      (r"show\s+(hidden\s+)?files", "ShowHiddenFiles"),
      (r"hide\s+hidden\s+files", "HideHiddenFiles"),
      // Codeshow.
      (r"(prev|previous)\s+slide", "CodeshowPrevSlide"),
      (r"(next|advance)\s+(the\s+)?slide", "CodeshowNextSlide"),
      // Settings.
      (r"(open|show)\s+(the\s+)?settings", "OpenSettings"),
      // Edit operations.
      (r"undo", "Undo"),
      (r"redo", "Redo"),
    ];

    patterns
      .into_iter()
      .filter_map(|(pattern, action)| {
        CommandPattern::new(pattern, action)
          .map_err(|e| {
            log::error!("Invalid regex '{pattern}' for '{action}': {e}");
            e
          })
          .ok()
      })
      .collect()
  }
}

impl Default for VoiceIntentParser {
  fn default() -> Self {
    Self::new()
  }
}

/// Command pattern for regex matching.
#[derive(Clone)]
struct CommandPattern {
  regex: Regex,
  action_name: String,
}

impl CommandPattern {
  fn new(pattern: &str, action_name: &str) -> Result<Self, regex::Error> {
    Ok(Self {
      regex: Regex::new(pattern)?,
      action_name: action_name.to_string(),
    })
  }
}
