//! Elixir symbol extraction for code folding.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for Elixir symbol extraction.
const ELIXIR_SYMBOLS_QUERY: &str = r#"
  ; Calls with do blocks (defmodule, def, defp, if, case, etc.)
  (call
    target: (identifier) @call_name
    (do_block) @do_blk) @call_with_do

  ; Anonymous functions
  (anonymous_function) @anon_fn

  ; Data structures
  (map) @map
  (list) @list
  (tuple) @tuple
"#;

static ELIXIR_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_elixir::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static ELIXIR_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_elixir::LANGUAGE.into(), ELIXIR_SYMBOLS_QUERY)
    .unwrap()
});

/// Extract symbols from Elixir source code for folding.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = ELIXIR_PARSER.lock().unwrap();

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
  let query = &*ELIXIR_SYMBOLS_Q;
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
        "call_name" => {
          let call_keyword = capture
            .node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string();

          // Extract the function/module name from arguments if available
          if let Some(parent) = capture.node.parent()
            && parent.kind() == "call"
          {
            if let Some(args) = parent.child_by_field_name("arguments") {
              // Get the first argument which is typically the function name
              if let Some(first_arg) = args.named_child(0) {
                let arg_text =
                  first_arg.utf8_text(source.as_bytes()).unwrap_or("");

                // For calls like (identifier) get just the name
                if first_arg.kind() == "call" {
                  if let Some(target) = first_arg.child_by_field_name("target")
                  {
                    name = format!(
                      "{call_keyword} {}",
                      target.utf8_text(source.as_bytes()).unwrap_or("")
                    );
                  } else {
                    name = format!("{} {}", call_keyword, arg_text);
                  }
                } else if matches!(first_arg.kind(), "identifier" | "alias") {
                  name = format!("{call_keyword} {arg_text}");
                } else {
                  name = call_keyword.clone();
                }
              } else {
                name = call_keyword.clone();
              }
            } else {
              name = call_keyword.clone();
            }
          }

          // Determine symbol kind based on the call keyword
          kind = match call_keyword.as_str() {
            "defmodule" => SymbolKind::Module,
            "def" | "defp" | "defmacro" | "defmacrop" => SymbolKind::Function,
            "defstruct" | "defexception" => SymbolKind::Struct,
            "defprotocol" | "defimpl" => SymbolKind::Module,
            "test" | "describe" => SymbolKind::Function,
            "if" | "unless" | "case" | "cond" | "try" | "receive" | "with" => {
              SymbolKind::Module
            }
            _ => SymbolKind::Function,
          };

          if name.is_empty() {
            name = call_keyword;
          }
        }
        "call_with_do" => {
          node = capture.node;
        }
        "anon_fn" => {
          node = capture.node;
          kind = SymbolKind::Function;
          name = "fn".to_string();
        }
        "map" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          name = "%{}".to_string();
        }
        "list" => {
          node = capture.node;
          kind = SymbolKind::Enum;
          name = "[]".to_string();
        }
        "tuple" => {
          node = capture.node;
          kind = SymbolKind::Struct;
          name = "{}".to_string();
        }
        "do_blk" => {
          // Skip, we use the parent call node
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
      fold_end_inclusive: false, // Elixir: show `end` keyword (like Python)
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
