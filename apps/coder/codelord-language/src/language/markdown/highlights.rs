//! Markdown syntax highlighting using tree-sitter-md.
//!
//! Uses both block and inline grammars with built-in highlight queries.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static BLOCK_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_md::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static INLINE_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static BLOCK_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_md::LANGUAGE.into(),
    tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
  )
  .unwrap()
});

static INLINE_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_md::INLINE_LANGUAGE.into(),
    tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
  )
  .unwrap()
});

/// Parse Markdown source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut tokens = Vec::new();

  // Parse block structure
  {
    let mut parser = BLOCK_PARSER.lock().unwrap();
    if let Some(tree) = parser.parse(source, None) {
      extract_tokens(&tree, source, &BLOCK_QUERY, &mut tokens);
    }
  }

  // Parse inline content
  {
    let mut parser = INLINE_PARSER.lock().unwrap();
    if let Some(tree) = parser.parse(source, None) {
      extract_tokens(&tree, source, &INLINE_QUERY, &mut tokens);
    }
  }

  // Sort by position
  tokens.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));

  // Remove duplicates
  tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end);
  tokens
}

fn extract_tokens(
  tree: &tree_sitter::Tree,
  source: &str,
  query: &Query,
  tokens: &mut Vec<Token>,
) {
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
}

fn map_capture_to_token(capture_name: &str) -> Option<TokenKind> {
  Some(match capture_name {
    "markup.heading" | "markup.heading.1" | "markup.heading.2"
    | "markup.heading.3" | "markup.heading.4" | "markup.heading.5"
    | "markup.heading.6" => TokenKind::IdentifierType,
    "markup.heading.marker" | "punctuation.special" => TokenKind::Keyword,
    "markup.italic" => TokenKind::Identifier,
    "markup.bold" | "markup.strong" => TokenKind::IdentifierConstant,
    "markup.strikethrough" => TokenKind::Comment,
    "markup.raw" | "markup.raw.inline" | "markup.raw.block" => {
      TokenKind::LiteralString
    }
    "markup.raw.delimiter" => TokenKind::Punctuation,
    "markup.link" | "markup.link.label" => TokenKind::IdentifierFunction,
    "markup.link.url" | "string.special.url" => TokenKind::LiteralString,
    "markup.list" | "markup.list.numbered" | "markup.list.unnumbered" => {
      TokenKind::Punctuation
    }
    "punctuation.special.markdown" => TokenKind::Keyword,
    "markup.quote" => TokenKind::Comment,
    "punctuation.delimiter" => TokenKind::Punctuation,
    "label" | "tag" => TokenKind::Attribute,
    "punctuation" => TokenKind::Punctuation,
    _ => return None,
  })
}
