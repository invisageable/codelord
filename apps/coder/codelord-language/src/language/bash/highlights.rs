//! Bash syntax highlighting using tree-sitter-bash.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static BASH_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_bash::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static BASH_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_bash::LANGUAGE.into(),
    tree_sitter_bash::HIGHLIGHT_QUERY,
  )
  .unwrap()
});

/// Parse Bash source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = BASH_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*BASH_QUERY;
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
    "keyword"
    | "keyword.control"
    | "keyword.conditional"
    | "keyword.repeat"
    | "keyword.function"
    | "keyword.return" => TokenKind::Keyword,
    "variable" | "variable.builtin" | "variable.parameter"
    | "variable.special" => TokenKind::Identifier,
    "function" | "function.builtin" | "function.call" => {
      TokenKind::IdentifierFunction
    }
    "constant" | "constant.builtin" => TokenKind::IdentifierConstant,
    "string" | "string.special" | "string.escape" => TokenKind::LiteralString,
    "number" => TokenKind::LiteralNumber,
    "comment" => TokenKind::Comment,
    "operator" => TokenKind::Operator,
    "punctuation.bracket" => TokenKind::PunctuationBracket,
    "punctuation.delimiter" | "punctuation.special" => TokenKind::Punctuation,
    _ => return None,
  })
}
