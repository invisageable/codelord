//! JSON symbol extraction for code folding.
//!
//! Extracts objects and arrays as foldable regions.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for JSON symbol extraction (objects and arrays).
const JSON_SYMBOLS_QUERY: &str = r#"
  (pair
    key: (string) @key
    value: (object) @object)

  (pair
    key: (string) @key
    value: (array) @array)

  (object) @root_object
  (array) @root_array
"#;

static JSON_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_json::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static JSON_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_json::LANGUAGE.into(), JSON_SYMBOLS_QUERY).unwrap()
});

/// Extract symbols from JSON source code for folding.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = JSON_PARSER.lock().unwrap();

  let Some(tree) = parser.parse(source, None) else {
    return SymbolMap::new(generation);
  };

  extract_symbols_from_tree(&tree, source, generation)
}

/// Extract symbols from a pre-parsed tree.
pub fn extract_symbols_from_tree(
  tree: &Tree,
  source: &str,
  generation: u64,
) -> SymbolMap {
  let mut map = SymbolMap::new(generation);
  let query = &*JSON_SYMBOLS_Q;
  let mut cursor = QueryCursor::new();
  let root = tree.root_node();
  let mut matches = cursor.matches(query, root, source.as_bytes());

  while let Some(matched) = matches.next() {
    if matched.captures.is_empty() {
      continue;
    }

    // Pattern 0 & 1: pair with key and object/array value
    // Pattern 2 & 3: standalone object/array (root level)
    let (node, name, kind) = match matched.pattern_index {
      0 => {
        // pair -> key (string), object
        let key_node = matched.captures[0].node;
        let obj_node = matched.captures[1].node;
        let key_text = key_node.utf8_text(source.as_bytes()).unwrap_or("{}");
        // Remove quotes from key
        let name = key_text.trim_matches('"').to_string();
        (obj_node, name, SymbolKind::Struct)
      }
      1 => {
        // pair -> key (string), array
        let key_node = matched.captures[0].node;
        let arr_node = matched.captures[1].node;
        let key_text = key_node.utf8_text(source.as_bytes()).unwrap_or("[]");
        let name = key_text.trim_matches('"').to_string();
        (arr_node, name, SymbolKind::Enum)
      }
      2 => {
        // root object
        let node = matched.captures[0].node;
        // Skip if this object is a value in a pair (already handled)
        if node.parent().map(|p| p.kind()) == Some("pair") {
          continue;
        }
        (node, "{}".to_string(), SymbolKind::Struct)
      }
      3 => {
        // root array
        let node = matched.captures[0].node;
        // Skip if this array is a value in a pair (already handled)
        if node.parent().map(|p| p.kind()) == Some("pair") {
          continue;
        }
        (node, "[]".to_string(), SymbolKind::Enum)
      }
      _ => continue,
    };

    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    // Only add if it spans multiple lines (foldable)
    if end_line <= start_line {
      continue;
    }

    let (display_text, highlight_ranges) =
      build_symbol_display_with_highlights(&kind, &name);

    map.add(SymbolAnchor {
      kind,
      line: start_line,
      col: node.start_position().column,
      name,
      byte_range: node.start_byte()..node.end_byte(),
      status: SymbolStatus::Default,
      end_line,
      is_foldable: true,
      fold_end_inclusive: false, // JSON: show closing brace/bracket
      display_text,
      highlight_ranges,
    });
  }

  map.sort();
  map
}

/// Build display text and highlight ranges for breadcrumb rendering.
fn build_symbol_display_with_highlights(
  kind: &SymbolKind,
  name: &str,
) -> (String, Vec<(Range<usize>, u8)>) {
  let display_text = name.to_string();
  let name_len = name.len();

  let name_token = match kind {
    SymbolKind::Struct => TokenKind::Identifier,
    SymbolKind::Enum => TokenKind::Identifier,
    _ => TokenKind::Identifier,
  };

  (display_text, vec![(0..name_len, name_token as u8)])
}
