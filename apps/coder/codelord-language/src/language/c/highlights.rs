//! C syntax highlighting using tree-sitter-c.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static C_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_c::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static C_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_c::LANGUAGE.into(),
    tree_sitter_c::HIGHLIGHT_QUERY,
  )
  .unwrap()
});

/// Parse C source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = C_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*C_QUERY;
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
    | "keyword.type"
    | "keyword.storage"
    | "keyword.modifier"
    | "keyword.control"
    | "keyword.return"
    | "keyword.conditional"
    | "keyword.repeat"
    | "keyword.directive" => TokenKind::Keyword,
    "variable" | "variable.builtin" | "variable.parameter" => {
      TokenKind::Identifier
    }
    "function" | "function.builtin" | "function.macro" => {
      TokenKind::IdentifierFunction
    }
    "type" | "type.builtin" | "type.definition" => TokenKind::IdentifierType,
    "constant" | "constant.builtin" | "constant.macro" => {
      TokenKind::IdentifierConstant
    }
    "string" | "string.special" | "string.escape" => TokenKind::LiteralString,
    "character" => TokenKind::LiteralChar,
    "number" | "number.float" => TokenKind::LiteralNumber,
    "comment" => TokenKind::Comment,
    "operator" => TokenKind::Operator,
    "punctuation.bracket" => TokenKind::PunctuationBracket,
    "punctuation.delimiter" => TokenKind::Punctuation,
    "property" | "field" => TokenKind::Identifier,
    "label" => TokenKind::Identifier,
    "include" | "preproc" => TokenKind::Keyword,
    _ => return None,
  })
}
