//! Zo language symbol extraction using tree-sitter-zo.
//!
//! Provides code folding and navigation via tree-sitter queries.

use codelord_core::symbol::{
  SymbolAnchor, SymbolKind, SymbolMap, SymbolStatus,
};
use codelord_core::token::TokenKind;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

use std::ops::Range;
use std::sync::{LazyLock, Mutex};

/// Query for Zo symbol extraction.
const ZO_SYMBOLS_QUERY: &str = r#"
  (function_declaration name: (identifier) @func.name) @func
  (struct_declaration name: (identifier) @struct.name) @struct
  (enum_declaration name: (identifier) @enum.name) @enum
  (apply_declaration trait: (identifier) @impl.type) @impl
  (abstract_declaration name: (identifier) @trait.name) @trait
  (pack_declaration name: (identifier) @mod.name) @mod
  (load_declaration) @import
  (val_declaration name: (identifier) @const.name) @const
  (type_declaration name: (identifier) @type.name) @type
"#;

static ZO_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_zo::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static ZO_SYMBOLS_Q: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_zo::LANGUAGE.into(), ZO_SYMBOLS_QUERY).unwrap()
});

/// Extract symbols from Zo source code (parses internally).
/// Use `extract_symbols_from_tree` if you already have a parsed tree.
pub fn extract_symbols(source: &str, generation: u64) -> SymbolMap {
  if source.len() > 1_000_000 {
    return SymbolMap::new(generation);
  }

  let mut parser = ZO_PARSER.lock().unwrap();

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
  let query = &*ZO_SYMBOLS_Q;
  let mut cursor = QueryCursor::new();
  let root = tree.root_node();
  let mut matches = cursor.matches(query, root, source.as_bytes());

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
      // For imports without names
      match matched.pattern_index {
        6 => "import".to_string(),
        3 => "impl".to_string(),
        _ => "unknown".to_string(),
      }
    };

    // Determine kind from pattern index (order matches query)
    let kind = match matched.pattern_index {
      0 => SymbolKind::Function, // function_declaration
      1 => SymbolKind::Struct,   // struct_declaration
      2 => SymbolKind::Enum,     // enum_declaration
      3 => SymbolKind::Impl,     // apply_declaration
      4 => SymbolKind::Trait,    // abstract_declaration
      5 => SymbolKind::Module,   // pack_declaration
      6 => SymbolKind::Import,   // load_declaration
      7 => SymbolKind::Const,    // val_declaration
      8 => SymbolKind::Struct,   // type_declaration (use Struct kind)
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
      fold_end_inclusive: false, // Zo: show closing brace
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
