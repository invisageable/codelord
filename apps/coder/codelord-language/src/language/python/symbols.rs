//! Python symbol extraction for code folding.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for Python symbol extraction.
const PYTHON_SYMBOLS_QUERY: &str = r#"
  ; Functions
  (function_definition
    name: (identifier) @fn_name) @function

  ; Classes
  (class_definition
    name: (identifier) @class_name) @class

  ; Control flow
  (if_statement) @if_stmt
  (for_statement) @for_stmt
  (while_statement) @while_stmt
  (try_statement) @try_stmt
  (with_statement) @with_stmt
  (match_statement) @match_stmt

  ; Data structures
  (dictionary) @dict
  (list) @list
"#;

static PYTHON_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_python::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static PYTHON_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_python::LANGUAGE.into(), PYTHON_SYMBOLS_QUERY)
    .unwrap()
});

/// Extract symbols from Python source code for folding.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = PYTHON_PARSER.lock().unwrap();

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
  let query = &*PYTHON_SYMBOLS_Q;
  let mut cursor = QueryCursor::new();
  let root = tree.root_node();
  let mut matches = cursor.matches(query, root, source.as_bytes());

  let capture_names = query.capture_names();

  while let Some(matched) = matches.next() {
    if matched.captures.is_empty() {
      continue;
    }

    let mut name = String::new();
    let mut node = matched.captures[0].node;
    let mut kind = SymbolKind::Function;

    for capture in matched.captures {
      let cap_name = &capture_names[capture.index as usize];

      match *cap_name {
        "fn_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        }
        "class_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("class")
            .to_string();
          kind = SymbolKind::Struct;
        }
        "function" => {
          node = capture.node;
          kind = SymbolKind::Function;
          if name.is_empty() {
            name = "def".to_string();
          }
        }
        "class" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          if name.is_empty() {
            name = "class".to_string();
          }
        }
        "if_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "if".to_string();
        }
        "for_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "for".to_string();
        }
        "while_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "while".to_string();
        }
        "try_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "try".to_string();
        }
        "with_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "with".to_string();
        }
        "match_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "match".to_string();
        }
        "dict" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          name = "{}".to_string();
        }
        "list" => {
          node = capture.node;
          kind = SymbolKind::Enum;
          name = "[]".to_string();
        }
        _ => {}
      }
    }

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
      fold_end_inclusive: true, // Python: hide end line (no closing brace)
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
    SymbolKind::Function => TokenKind::IdentifierFunction,
    SymbolKind::Struct => TokenKind::IdentifierType,
    SymbolKind::Enum => TokenKind::Identifier,
    SymbolKind::Module => TokenKind::Keyword,
    _ => TokenKind::Identifier,
  };

  (display_text, vec![(0..name_len, name_token as u8)])
}
