//! JavaScript symbol extraction for code folding.
//!
//! Extracts functions, classes, objects, and arrays as foldable regions.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for JavaScript symbol extraction.
const JS_SYMBOLS_QUERY: &str = r#"
  ; Functions
  (function_declaration
    name: (identifier) @fn_name) @function

  ; Arrow functions assigned to variables
  (lexical_declaration
    (variable_declarator
      name: (identifier) @arrow_name
      value: (arrow_function) @arrow_body))

  ; Methods in objects/classes
  (method_definition
    name: (property_identifier) @method_name) @method

  ; Classes (class_declaration for `class Foo {}`, class for `const Foo = class {}`)
  (class_declaration
    name: (identifier) @class_name) @class_decl

  (class
    name: (identifier)? @class_expr_name) @class_expr

  ; Objects
  (object) @object

  ; Arrays
  (array) @array

  ; Control flow (for folding)
  (if_statement) @if_stmt
  (for_statement) @for_stmt
  (for_in_statement) @for_in_stmt
  (while_statement) @while_stmt
  (try_statement) @try_stmt
  (switch_statement) @switch_stmt
"#;

static JS_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_javascript::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static JS_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_javascript::LANGUAGE.into(), JS_SYMBOLS_QUERY)
    .unwrap()
});

/// Extract symbols from JavaScript source code for folding.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = JS_PARSER.lock().unwrap();

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
  let query = &*JS_SYMBOLS_Q;
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
        "fn_name" | "arrow_name" | "method_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        }
        "class_name" | "class_expr_name" => {
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
            name = "function".to_string();
          }
        }
        "arrow_body" => {
          node = capture.node;
          kind = SymbolKind::Function;
        }
        "method" => {
          node = capture.node;
          kind = SymbolKind::Function;
        }
        "class_decl" | "class_expr" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          if name.is_empty() {
            name = "class".to_string();
          }
        }
        "object" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          // Check if object is assigned to a variable
          if let Some(parent) = node.parent()
            && parent.kind() == "variable_declarator"
            && let Some(name_node) = parent.child_by_field_name("name")
          {
            name = name_node
              .utf8_text(source.as_bytes())
              .unwrap_or("{}")
              .to_string();
          }

          if name.is_empty() {
            name = "{}".to_string();
          }
        }
        "array" => {
          node = capture.node;
          kind = SymbolKind::Enum;
          name = "[]".to_string();
        }
        "if_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "if".to_string();
        }
        "for_stmt" | "for_in_stmt" => {
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
        "switch_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "switch".to_string();
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
      fold_end_inclusive: false, // JS: show closing brace
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
