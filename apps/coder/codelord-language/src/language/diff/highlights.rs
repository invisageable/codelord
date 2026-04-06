//! Diff/patch syntax highlighting using tree-sitter-diff.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static DIFF_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_diff::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static DIFF_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_diff::LANGUAGE.into(),
    tree_sitter_diff::HIGHLIGHTS_QUERY,
  )
  .unwrap()
});

/// Parse diff source and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = DIFF_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*DIFF_QUERY;
  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

  while let Some(match_) = matches.next() {
    for capture in match_.captures {
      let node = capture.node;
      let capture_name = &query.capture_names()[capture.index as usize];

      let kind = map_capture_to_token(capture_name);
      if let Some(kind) = kind {
        tokens.push(Token {
          kind,
          start: node.start_byte(),
          end: node.end_byte(),
        });
      }
    }
  }

  tokens.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));
  tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end);
  tokens
}

fn map_capture_to_token(capture_name: &str) -> Option<TokenKind> {
  Some(match capture_name {
    "diff.plus" | "addition" | "inserted" => TokenKind::LiteralString,
    "diff.minus" | "deletion" | "deleted" => TokenKind::Error,
    "diff.header" | "header" | "filename" => TokenKind::IdentifierFunction,
    "diff.hunk" | "hunk" | "range" => TokenKind::Keyword,
    "context" => TokenKind::Comment,
    "number" => TokenKind::LiteralNumber,
    "commit" | "hash" => TokenKind::IdentifierConstant,
    "author" | "date" => TokenKind::Identifier,
    _ => return None,
  })
}
