//! Python syntax highlighting using tree-sitter-python.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static PY_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_python::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static PY_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_python::LANGUAGE.into(),
    tree_sitter_python::HIGHLIGHTS_QUERY,
  )
  .unwrap()
});

/// Parse Python source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = PY_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*PY_QUERY;
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
    | "keyword.exception"
    | "keyword.operator" => TokenKind::Keyword,
    "variable" | "variable.builtin" | "variable.parameter" => {
      TokenKind::Identifier
    }
    "function" | "function.builtin" | "function.method" | "function.call" => {
      TokenKind::IdentifierFunction
    }
    "type" | "type.builtin" => TokenKind::IdentifierType,
    "constructor" | "class" => TokenKind::IdentifierType,
    "constant" | "constant.builtin" => TokenKind::IdentifierConstant,
    "boolean" => TokenKind::LiteralBool,
    "string" | "string.special" | "string.escape" | "string.documentation" => {
      TokenKind::LiteralString
    }
    "number" | "number.float" => TokenKind::LiteralNumber,
    "comment" => TokenKind::Comment,
    "operator" => TokenKind::Operator,
    "punctuation.bracket" => TokenKind::PunctuationBracket,
    "punctuation.delimiter" => TokenKind::Punctuation,
    "attribute" | "decorator" => TokenKind::Attribute,
    "property" => TokenKind::Identifier,
    "module" | "namespace" => TokenKind::Namespace,
    "self" => TokenKind::SpecialSelf,
    _ => return None,
  })
}
