use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;

/// Query for Rust symbol extraction.
const RUST_SYMBOLS_QUERY: &str = r#"
  (function_item name: (identifier) @func.name) @func
  (struct_item name: (type_identifier) @struct.name) @struct
  (enum_item name: (type_identifier) @enum.name) @enum
  (impl_item type: (_) @impl.type) @impl
  (trait_item name: (type_identifier) @trait.name) @trait
  (use_declaration) @import
  (const_item name: (identifier) @const.name) @const
  (mod_item name: (identifier) @mod.name) @mod
"#;

/// Extract symbols from Rust source code (parses internally).
/// Use `extract_symbols_from_tree` if you already have a parsed tree.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  let mut parser = Parser::new();
  let language = tree_sitter_rust::LANGUAGE.into();

  if parser.set_language(&language).is_err() {
    return SymbolMap::new(generation);
  }

  let Some(tree) = parser.parse(source, None) else {
    return SymbolMap::new(generation);
  };

  extract_symbols_from_tree(&tree, source, generation)
}

/// Extract symbols from a pre-parsed tree.
/// Use this when sharing a tree with highlighting (avoids double-parsing).
pub fn extract_symbols_from_tree(
  tree: &Tree,
  source: &str,
  generation: u64,
) -> SymbolMap {
  let mut map = SymbolMap::new(generation);
  let language = tree_sitter_rust::LANGUAGE.into();

  let Ok(query) = Query::new(&language, RUST_SYMBOLS_QUERY) else {
    return map;
  };

  let mut cursor = QueryCursor::new();
  let root = tree.root_node();
  let mut matches = cursor.matches(&query, root, source.as_bytes());

  while let Some(matched) = matches.next() {
    if matched.captures.is_empty() {
      continue;
    }

    let node = matched.captures[0].node;
    let name_capture = matched.captures.get(1);

    let name = if let Some(name_cap) = name_capture {
      name_cap
        .node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .to_string()
    } else {
      // For imports and impls without names
      match matched.pattern_index {
        5 => "import".to_string(),
        3 => "impl".to_string(),
        _ => "unknown".to_string(),
      }
    };

    // Determine kind from pattern index
    let kind = match matched.pattern_index {
      0 => SymbolKind::Function,
      1 => SymbolKind::Struct,
      2 => SymbolKind::Enum,
      3 => SymbolKind::Impl,
      4 => SymbolKind::Trait,
      5 => SymbolKind::Import,
      6 => SymbolKind::Const,
      7 => SymbolKind::Module,
      _ => continue,
    };

    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    // Build display text with syntax highlights
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
      is_foldable: end_line > start_line,
      fold_end_inclusive: false, // Rust: show closing brace
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
  let keyword = kind.keyword();
  let display_text = format!("{keyword} {name}");

  let keyword_len = keyword.len();
  let name_start = keyword_len + 1;
  let name_end = name_start + name.len();

  let name_token = match kind {
    SymbolKind::Function => TokenKind::IdentifierFunction,
    SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait => {
      TokenKind::IdentifierType
    }
    SymbolKind::Impl => TokenKind::IdentifierType,
    SymbolKind::Module => TokenKind::Namespace,
    SymbolKind::Import => TokenKind::Namespace,
    SymbolKind::Const => TokenKind::IdentifierConstant,
  };

  (
    display_text,
    vec![
      (0..keyword_len, TokenKind::Keyword as u8),
      (name_start..name_end, name_token as u8),
    ],
  )
}
