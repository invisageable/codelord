//! JSON syntax highlighting using tree-sitter-json.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

use std::sync::{LazyLock, Mutex};

const HIGHLIGHTS_QUERY: &str = r#"
; Object keys (strings in pair position)
(pair
  key: (string) @property)

; String values
(string) @string

; Numbers
(number) @number

; Booleans
(true) @boolean
(false) @boolean

; Null
(null) @constant

; Punctuation
["{" "}" "[" "]"] @punctuation.bracket
[":" ","] @punctuation.delimiter
"#;

static JSON_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_json::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static JSON_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_json::LANGUAGE.into(), HIGHLIGHTS_QUERY).unwrap()
});

/// Parse JSON source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = JSON_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*JSON_QUERY;
  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

  while let Some(match_) = matches.next() {
    for capture in match_.captures {
      let node = capture.node;
      let capture_name = &query.capture_names()[capture.index as usize];

      let kind = match *capture_name {
        "property" => TokenKind::Identifier,
        "string" => TokenKind::LiteralString,
        "number" => TokenKind::LiteralNumber,
        "boolean" => TokenKind::LiteralBool,
        "constant" => TokenKind::IdentifierConstant,
        "punctuation.delimiter" => TokenKind::Punctuation,
        "punctuation.bracket" => {
          let depth = count_bracket_depth(node);

          match depth % 3 {
            0 => TokenKind::BracketLevel0,
            1 => TokenKind::BracketLevel1,
            _ => TokenKind::BracketLevel2,
          }
        }
        _ => continue,
      };

      tokens.push(Token {
        kind,
        start: node.start_byte(),
        end: node.end_byte(),
      });
    }
  }

  // Sort by position, property keys should override generic strings
  tokens.sort_by(|a, b| {
    a.start
      .cmp(&b.start)
      .then_with(|| a.end.cmp(&b.end).reverse())
  });

  // Remove duplicates, keeping first (property over string)
  tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end);
  tokens
}

/// Count bracket nesting depth by walking up the tree.
fn count_bracket_depth(start_node: Node) -> usize {
  let mut depth = 0;
  let mut current_node = Some(start_node);

  while let Some(node) = current_node {
    let kind = node.kind();

    if kind == "object" || kind == "array" {
      depth += 1;
    }
    current_node = node.parent();
  }

  depth
}
