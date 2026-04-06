//! Search engine using Aho-Corasick for efficient pattern matching.
//!
//! This module implements the core search algorithms using Aho-Corasick
//! for plain text search and regex for pattern matching.

use super::SearchState;

use aho_corasick::AhoCorasick;
use regex::RegexBuilder;
use ropey::Rope;

use std::ops::Range;

/// Perform search on a Rope without allocating the entire content as a String.
/// This is the optimized, zero-copy version that works directly on the rope.
pub fn perform_search(rope: &Rope, state: &SearchState) -> Vec<Range<usize>> {
  if state.query.is_empty() {
    return Vec::with_capacity(0);
  }

  if state.regex_mode {
    // Regex still needs the full string, but we can optimize later
    let content = rope.to_string();
    perform_regex_search(&content, state)
  } else {
    perform_plain_search(rope, state)
  }
}

/// Perform search on a string (for TextBuffer compatibility).
pub fn perform_search_str(
  content: &str,
  state: &SearchState,
) -> Vec<Range<usize>> {
  if state.query.is_empty() {
    return Vec::with_capacity(0);
  }

  if state.regex_mode {
    perform_regex_search(content, state)
  } else {
    perform_plain_search_str(content, state)
  }
}

/// Perform regex-based search.
fn perform_regex_search(
  content: &str,
  state: &SearchState,
) -> Vec<Range<usize>> {
  match RegexBuilder::new(&state.query)
    .case_insensitive(!state.case_sensitive)
    .build()
  {
    Ok(re) => re.find_iter(content).map(|m| m.range()).collect(),
    Err(_) => Vec::with_capacity(0),
  }
}

/// Perform plain text search on a Rope using Aho-Corasick algorithm.
/// This version works directly on the rope without allocating a full string.
fn perform_plain_search(rope: &Rope, state: &SearchState) -> Vec<Range<usize>> {
  let ac = match AhoCorasick::builder()
    .ascii_case_insensitive(!state.case_sensitive)
    .match_kind(aho_corasick::MatchKind::LeftmostFirst)
    .build([&state.query])
  {
    Ok(ac) => ac,
    Err(_) => return Vec::with_capacity(0),
  };

  let mut matches = Vec::new();
  let mut byte_offset = 0;

  // Iterate over rope chunks without allocating
  for chunk in rope.chunks() {
    // Find matches in this chunk
    for mat in ac.find_iter(chunk.as_bytes()) {
      let global_start = byte_offset + mat.start();
      let global_end = byte_offset + mat.end();

      if state.whole_word {
        // For word boundary checking, we need to look at the characters
        // Convert byte positions to char positions for boundary check
        let char_start = rope.byte_to_char(global_start);
        let char_end = rope.byte_to_char(global_end);

        if is_word_boundary_rope(rope, char_start, char_end) {
          matches.push(global_start..global_end);
        }
      } else {
        matches.push(global_start..global_end);
      }
    }

    byte_offset += chunk.len();
  }

  matches
}

/// Perform plain text search on a string using Aho-Corasick algorithm.
fn perform_plain_search_str(
  content: &str,
  state: &SearchState,
) -> Vec<Range<usize>> {
  let ac = match AhoCorasick::builder()
    .ascii_case_insensitive(!state.case_sensitive)
    .match_kind(aho_corasick::MatchKind::LeftmostFirst)
    .build([&state.query])
  {
    Ok(ac) => ac,
    Err(_) => return Vec::with_capacity(0),
  };

  let mut matches = Vec::new();

  for mat in ac.find_iter(content.as_bytes()) {
    let start = mat.start();
    let end = mat.end();

    if state.whole_word {
      if is_word_boundary_str(content, start, end) {
        matches.push(start..end);
      }
    } else {
      matches.push(start..end);
    }
  }

  matches
}

/// Check if a match is at word boundaries in a Rope.
fn is_word_boundary_rope(
  rope: &Rope,
  char_start: usize,
  char_end: usize,
) -> bool {
  let start_boundary = if char_start == 0 {
    true
  } else {
    let prev_char_idx = char_start - 1;
    !rope.char(prev_char_idx).is_alphanumeric()
  };

  let end_boundary = if char_end >= rope.len_chars() {
    true
  } else {
    !rope.char(char_end).is_alphanumeric()
  };

  start_boundary && end_boundary
}

/// Check if a match is at word boundaries in a string.
fn is_word_boundary_str(content: &str, start: usize, end: usize) -> bool {
  let start_boundary = if start == 0 {
    true
  } else {
    content[..start]
      .chars()
      .last()
      .map(|c| !c.is_alphanumeric())
      .unwrap_or(true)
  };

  let end_boundary = if end >= content.len() {
    true
  } else {
    content[end..]
      .chars()
      .next()
      .map(|c| !c.is_alphanumeric())
      .unwrap_or(true)
  };

  start_boundary && end_boundary
}

/// Validates regex pattern.
///
/// Returns error message if invalid.
pub fn validate_regex(pattern: &str) -> Option<String> {
  match regex::Regex::new(pattern) {
    Ok(_) => None,
    Err(error) => Some(format!("Invalid regex: {error}")),
  }
}
