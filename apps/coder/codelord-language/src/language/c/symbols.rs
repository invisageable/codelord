//! C symbol extraction for code folding.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for C symbol extraction.
const C_SYMBOLS_QUERY: &str = r#"
  ; Functions
  (function_definition
    declarator: (function_declarator
      declarator: (identifier) @fn_name)) @function

  ; Structs
  (struct_specifier
    name: (type_identifier) @struct_name
    body: (field_declaration_list)) @struct

  ; Enums
  (enum_specifier
    name: (type_identifier) @enum_name
    body: (enumerator_list)) @enum

  ; Unions
  (union_specifier
    name: (type_identifier) @union_name
    body: (field_declaration_list)) @union

  ; Control flow
  (if_statement) @if_stmt
  (for_statement) @for_stmt
  (while_statement) @while_stmt
  (do_statement) @do_stmt
  (switch_statement) @switch_stmt

  ; Compound statements (blocks)
  (compound_statement) @block
"#;

static C_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_c::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static C_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_c::LANGUAGE.into(), C_SYMBOLS_QUERY).unwrap()
});

/// Extract symbols from C source code for folding.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = C_PARSER.lock().unwrap();

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
  let query = &*C_SYMBOLS_Q;
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
        "function" => {
          node = capture.node;
          kind = SymbolKind::Function;
        }
        "struct_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        }
        "struct" => {
          node = capture.node;
          kind = SymbolKind::Struct;
        }
        "enum_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        }
        "enum" => {
          node = capture.node;
          kind = SymbolKind::Enum;
        }
        "union_name" => {
          name = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();
        }
        "union" => {
          node = capture.node;
          kind = SymbolKind::Struct; // Treat union like struct
          if name.is_empty() {
            name = "union".to_string();
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
        "do_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "do".to_string();
        }
        "switch_stmt" => {
          node = capture.node;
          kind = SymbolKind::Module;
          name = "switch".to_string();
        }
        "block" => {
          // Skip blocks that are children of other constructs
          if let Some(parent) = capture.node.parent() {
            let pk = parent.kind();
            if pk == "function_definition"
              || pk == "if_statement"
              || pk == "for_statement"
              || pk == "while_statement"
              || pk == "do_statement"
              || pk == "switch_statement"
            {
              continue;
            }
          }
          node = capture.node;
          kind = SymbolKind::Module;
          name = "block".to_string();
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

    // Set default name for unnamed symbols
    if name.is_empty() {
      name = match kind {
        SymbolKind::Function => "function".to_string(),
        SymbolKind::Struct => "struct".to_string(),
        SymbolKind::Enum => "enum".to_string(),
        _ => "block".to_string(),
      };
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
      fold_end_inclusive: false, // C: show closing brace
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
    SymbolKind::Enum => TokenKind::IdentifierType,
    SymbolKind::Module => TokenKind::Keyword,
    _ => TokenKind::Identifier,
  };

  (display_text, vec![(0..name_len, name_token as u8)])
}
