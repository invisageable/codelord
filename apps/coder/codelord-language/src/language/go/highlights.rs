//! Go syntax highlighting using tree-sitter-go.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static GO_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_go::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static GO_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_go::LANGUAGE.into(),
    tree_sitter_go::HIGHLIGHTS_QUERY,
  )
  .unwrap()
});

/// Parse Go source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = GO_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*GO_QUERY;
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
    | "keyword.function"
    | "keyword.return"
    | "keyword.control"
    | "keyword.conditional"
    | "keyword.repeat"
    | "keyword.import"
    | "keyword.type"
    | "keyword.storage" => TokenKind::Keyword,
    "variable" | "variable.builtin" | "variable.parameter" => {
      TokenKind::Identifier
    }
    "function" | "function.builtin" | "function.method" | "function.call" => {
      TokenKind::IdentifierFunction
    }
    "type" | "type.builtin" | "type.definition" => TokenKind::IdentifierType,
    "constant" | "constant.builtin" => TokenKind::IdentifierConstant,
    "boolean" => TokenKind::LiteralBool,
    "string" | "string.special" | "string.escape" => TokenKind::LiteralString,
    "number" | "number.float" => TokenKind::LiteralNumber,
    "comment" => TokenKind::Comment,
    "operator" => TokenKind::Operator,
    "punctuation.bracket" => TokenKind::PunctuationBracket,
    "punctuation.delimiter" => TokenKind::Punctuation,
    "property" | "field" => TokenKind::Identifier,
    "label" => TokenKind::Identifier,
    "namespace" | "module" | "package" => TokenKind::Namespace,
    _ => return None,
  })
}
