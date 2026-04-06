use codelord_core::token::{Token, TokenKind};

use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

use std::cmp::Ordering;
use std::sync::{LazyLock, Mutex};

const HIGHLIGHTS_QUERY: &str = r#"
; Base identifiers (lowest priority)
(identifier) @variable
(type_identifier) @type
(primitive_type) @type.builtin
(field_identifier) @property

; Crate and module names in paths
(scoped_identifier
  path: (identifier) @namespace)

(scoped_type_identifier
  path: (identifier) @namespace)

(scoped_identifier
  path: (scoped_identifier
    name: (identifier) @namespace))

(use_declaration
  argument: (identifier) @namespace)

(use_declaration
  argument: (scoped_identifier
    path: (identifier) @namespace))

(use_declaration
  argument: (scoped_use_list
    path: (identifier) @namespace))

; Module paths in function calls
(call_expression
  function: (scoped_identifier
    path: (identifier) @namespace))

; Functions and method calls
(call_expression
  function: [
    (identifier) @function
    (scoped_identifier name: (identifier) @function)
    (field_expression field: (field_identifier) @function.method)
  ])

(function_item name: (identifier) @function)
(function_signature_item name: (identifier) @function)

; Macros
(macro_invocation
  macro: [
    (identifier) @function.special
    (scoped_identifier name: (identifier) @function.special)
  ])

; Brackets
["(" ")" "{" "}" "[" "]"] @punctuation.bracket

; Angle brackets (for generics)
(_
  .
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Punctuation
["." ";" "," "::"] @punctuation.delimiter

; Keywords (high priority - must come after identifiers to override)
[
  "as" "async" "await" "break" "const" "continue"
  "default" "dyn" "else" "enum" "extern" "fn"
  "for" "if" "impl" "in" "let" "loop"
  "macro_rules!" "match" "mod" "move" "pub" "raw"
  "ref" "return" "static" "struct" "trait" "type"
  "union" "unsafe" "use" "where" "while" "yield"
  (crate) (mutable_specifier) (super) (self)
] @keyword

; String literals
[(string_literal) (raw_string_literal) (char_literal)] @string

; Numeric literals
[(integer_literal) (float_literal)] @number

; Booleans
(boolean_literal) @boolean

; Comments - doc comments first (higher priority)
(line_comment (doc_comment)) @comment.documentation
(block_comment (doc_comment)) @comment.documentation

(line_comment) @comment
(block_comment) @comment

; Operators
[
  "!=" "%" "%=" "&" "&=" "&&"
  "*" "*=" "+" "+=" "-" "-=" "->"
  ".." "..=" "..." "/" "/=" ":"
  "<<" "<<=" "<" "<=" "=" "=="
  "=>" ">" ">=" ">>" ">>=" "@"
  "^" "^=" "|" "|=" "||" "?"
] @operator

; Special ! operator for macros
(unary_expression "!" @operator)

; Lifetime annotations
(lifetime) @lifetime

; Parameters
(parameter (identifier) @variable.parameter)

; Attributes
(attribute_item) @attribute
(inner_attribute_item) @attribute

; Identifier conventions
; Assume uppercase names are types/enum-constructors
((identifier) @type
 (#match? @type "^[A-Z]"))

; Assume all-caps names are constants
((identifier) @constant
 (#match? @constant "^_*[A-Z][A-Z0-9_]*$"))
"#;

static RUST_PARSER: LazyLock<Mutex<Parser>> = LazyLock::new(|| {
  let mut parser = Parser::new();

  parser
    .set_language(&tree_sitter_rust::LANGUAGE.into())
    .unwrap();

  Mutex::new(parser)
});

static RUST_QUERY: LazyLock<Query> = LazyLock::new(|| {
  Query::new(&tree_sitter_rust::LANGUAGE.into(), HIGHLIGHTS_QUERY).unwrap()
});

pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut parser = RUST_PARSER.lock().unwrap();

  let tree = match parser.parse(source, None) {
    Some(tree) => tree,
    None => return Vec::with_capacity(0),
  };

  // Query the entire tree for all highlights
  let mut tokens = Vec::new();
  let query = &*RUST_QUERY;
  let mut cursor = QueryCursor::new();
  let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());

  while let Some(match_) = matches.next() {
    for capture in match_.captures {
      let node = capture.node;
      let capture_name = &query.capture_names()[capture.index as usize];

      let kind = match *capture_name {
        "comment" | "comment.documentation" => TokenKind::Comment,
        "string" => TokenKind::LiteralString,
        "number" => TokenKind::LiteralNumber,
        "boolean" => TokenKind::LiteralBool,
        "function" | "function.method" | "function.special" => {
          TokenKind::IdentifierFunction
        }
        "type" | "type.builtin" => TokenKind::IdentifierType,
        "constant" => TokenKind::IdentifierConstant,
        "namespace" => TokenKind::Namespace,
        "keyword" => TokenKind::Keyword,
        "operator" => TokenKind::Operator,
        "variable" => TokenKind::Identifier,
        "variable.special" => TokenKind::SpecialSelf,
        "variable.parameter" => TokenKind::Identifier,
        "property" => TokenKind::Identifier,
        "lifetime" => TokenKind::SpecialSelf,
        "attribute" => TokenKind::Attribute,
        "punctuation.delimiter" => TokenKind::Punctuation,
        "punctuation.special" => TokenKind::Punctuation,
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

  // Sort tokens by position, then by priority (namespace > others)
  tokens.sort_by(|a, b| {
    a.start.cmp(&b.start).then_with(|| {
      // Higher priority types should come first
      match (&a.kind, &b.kind) {
        (TokenKind::Namespace, TokenKind::Namespace) => Ordering::Equal,
        (TokenKind::Namespace, _) => Ordering::Less,
        (_, TokenKind::Namespace) => Ordering::Greater,
        _ => Ordering::Equal,
      }
    })
  });

  // Remove duplicates, keeping the first (highest priority) token at each
  // position
  tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end);
  tokens
}

/// Walks up the syntax tree from a given bracket to count its nesting level.
fn count_bracket_depth(start_node: Node) -> usize {
  let mut depth = 0;
  let mut current_node = Some(start_node);

  while let Some(node) = current_node {
    let kind = node.kind();

    if kind.ends_with("_block")
      || kind.ends_with("_list")
      || kind == "arguments"
      || kind == "parameters"
      || kind == "array_expression"
    {
      depth += 1;
    }
    current_node = node.parent();
  }

  depth
}
