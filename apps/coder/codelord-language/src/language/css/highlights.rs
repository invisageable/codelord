//! CSS syntax highlighting using tree-sitter-css.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

static CSS_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();
  parser
    .set_language(&tree_sitter_css::LANGUAGE.into())
    .unwrap();
  Mutex::new(parser)
});

static CSS_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(
    &tree_sitter_css::LANGUAGE.into(),
    tree_sitter_css::HIGHLIGHTS_QUERY,
  )
  .unwrap()
});

/// Parse CSS source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = CSS_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*CSS_QUERY;
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
    "keyword" | "keyword.directive" => TokenKind::Keyword,
    "tag" | "type" => TokenKind::IdentifierType,
    "property" => TokenKind::Identifier,
    "variable" | "variable.parameter" => TokenKind::Identifier,
    "function" => TokenKind::IdentifierFunction,
    "constant" | "constant.builtin" => TokenKind::IdentifierConstant,
    "string" => TokenKind::LiteralString,
    "number" | "float" => TokenKind::LiteralNumber,
    "comment" => TokenKind::Comment,
    "operator" => TokenKind::Operator,
    "punctuation.bracket" => TokenKind::PunctuationBracket,
    "punctuation.delimiter" => TokenKind::Punctuation,
    "attribute" | "attribute.class" | "attribute.id" => TokenKind::Attribute,
    "selector" | "selector.class" | "selector.id" | "selector.tag" => {
      TokenKind::IdentifierFunction
    }
    _ => return None,
  })
}
