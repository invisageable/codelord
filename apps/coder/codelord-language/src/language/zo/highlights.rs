//! Zo language syntax highlighting using tree-sitter-zo.
//!
//! This provides accurate highlighting with error recovery and comment support.

use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

use std::cmp::Ordering;
use std::sync::{LazyLock, Mutex};

const HIGHLIGHTS_QUERY: &str = r##"
; Comments (highest priority for these)
(line_comment) @comment
(block_comment) @comment

; Literals
(integer_literal) @number
(float_literal) @number
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(bytes_literal) @string
(boolean_literal) @boolean

; Keywords
[
  "pack"
  "load"
  "type"
  "ext"
  "abstract"
  "apply"
  "fun"
  "fn"
  "val"
  "struct"
  "enum"
  "group"
  "state"
  "imu"
  "mut"
  "raw"
  "for"
  "while"
  "loop"
  "if"
  "else"
  "match"
  "when"
  "return"
  "nursery"
  "spawn"
  "await"
  "as"
  "is"
  "and"
] @keyword

(break_expression) @keyword
(continue_expression) @keyword

; Visibility
(visibility) @keyword.modifier

; Types
(primitive_type) @type.builtin
(self_type) @type.builtin
(generic_type) @type

; Function definitions
(function_declaration
  name: (identifier) @function)

(abstract_method
  name: (identifier) @function)

; Struct/enum/trait definitions
(struct_declaration
  name: (identifier) @type)

(enum_declaration
  name: (identifier) @type)

(abstract_declaration
  name: (identifier) @type)

(apply_declaration
  trait: (identifier) @type)

(type_declaration
  name: (identifier) @type)

; Enum variants
(enum_variant
  name: (identifier) @constant)

; Constants
(val_declaration
  name: (identifier) @constant)

; Module/pack
(pack_declaration
  name: (identifier) @namespace)

(module_path
  (identifier) @namespace)

; Parameters
(parameter
  name: (identifier) @variable.parameter)

; Fields
(field
  name: (identifier) @property)

(postfix_expression
  field: (identifier) @property)

; Function calls
(postfix_expression
  function: (primary_expression
    (identifier) @function.call))

; Attributes
(attribute
  name: (identifier) @attribute)

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "!"
  "&&"
  "||"
  "&"
  "|"
  "^"
  "<<"
  ">>"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  ".."
  "..="
  "|>"
] @operator

; Punctuation
[
  ","
  "."
  ";"
  ":"
  "::"
  ":="
  "::="
  "->"
  "=>"
  "%%"
  "#"
  "$"
] @punctuation.delimiter

; Brackets
["(" ")" "{" "}" "[" "]"] @punctuation.bracket

; Identifiers (lowest priority)
(identifier) @variable
"##;

static ZO_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_zo::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static ZO_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_zo::LANGUAGE.into(), HIGHLIGHTS_QUERY).unwrap()
});

/// Parse Zo source code and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = ZO_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  let mut tokens = Vec::new();
  let query = &*ZO_QUERY;
  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

  while let Some(match_) = matches.next() {
    for capture in match_.captures {
      let node = capture.node;
      let capture_name = &query.capture_names()[capture.index as usize];

      let kind = match *capture_name {
        "comment" => TokenKind::Comment,
        "string" => TokenKind::LiteralString,
        "number" => TokenKind::LiteralNumber,
        "boolean" => TokenKind::LiteralBool,
        "function" | "function.call" => TokenKind::IdentifierFunction,
        "type" | "type.builtin" => TokenKind::IdentifierType,
        "constant" => TokenKind::IdentifierConstant,
        "namespace" => TokenKind::Namespace,
        "keyword" | "keyword.modifier" => TokenKind::Keyword,
        "operator" => TokenKind::Operator,
        "variable" => TokenKind::Identifier,
        "variable.parameter" => TokenKind::Identifier,
        "property" => TokenKind::Identifier,
        "attribute" => TokenKind::Attribute,
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

  // Sort tokens by position, then by priority
  tokens.sort_by(|a, b| {
    a.start
      .cmp(&b.start)
      .then_with(|| match (&a.kind, &b.kind) {
        (TokenKind::Namespace, TokenKind::Namespace) => Ordering::Equal,
        (TokenKind::Namespace, _) => Ordering::Less,
        (_, TokenKind::Namespace) => Ordering::Greater,
        _ => Ordering::Equal,
      })
  });

  // Remove duplicates
  tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end);
  tokens
}

/// Count bracket nesting depth by walking up the tree.
fn count_bracket_depth(start_node: Node) -> usize {
  let mut depth = 0;
  let mut current_node = Some(start_node);

  while let Some(node) = current_node {
    let kind = node.kind();

    if kind == "block"
      || kind == "parameter_list"
      || kind == "argument_list"
      || kind == "array_expression"
      || kind == "tuple_expression"
      || kind == "generic_parameters"
      || kind == "field_list"
      || kind == "enum_variant_list"
    {
      depth += 1;
    }
    current_node = node.parent();
  }

  depth
}
