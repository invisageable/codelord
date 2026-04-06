//! Environment file (.env) syntax highlighting.
//!
//! Hand-written tokenizer for .env files which have a simple format:
//! - Lines starting with `#` are comments
//! - `KEY=VALUE` pairs
//! - Optional quoted values: `KEY="value"` or `KEY='value'`

use codelord_core::token::{Token, TokenKind};

/// Parse .env source and extract tokens for syntax highlighting.
pub fn parse(source: &str) -> Vec<Token> {
  if source.len() > 1_000_000 {
    return Vec::with_capacity(0);
  }

  let mut tokens = Vec::new();
  let mut offset = 0;

  for line in source.lines() {
    let line_start = offset;
    let trimmed = line.trim_start();
    let leading_ws = line.len() - trimmed.len();

    if trimmed.starts_with('#') {
      // Comment line
      tokens.push(Token {
        kind: TokenKind::Comment,
        start: line_start,
        end: line_start + line.len(),
      });
    } else if let Some(eq_pos) = trimmed.find('=') {
      let key_start = line_start + leading_ws;
      let key_end = key_start + eq_pos;
      let eq_start = key_end;
      let eq_end = eq_start + 1;
      let value_start = eq_end;
      let value_end = line_start + line.len();

      // Key (variable name)
      if eq_pos > 0 {
        tokens.push(Token {
          kind: TokenKind::IdentifierConstant,
          start: key_start,
          end: key_end,
        });
      }

      // Equals sign
      tokens.push(Token {
        kind: TokenKind::Operator,
        start: eq_start,
        end: eq_end,
      });

      // Value
      if value_start < value_end {
        tokens.push(Token {
          kind: TokenKind::LiteralString,
          start: value_start,
          end: value_end,
        });
      }
    }

    // Move to next line (+1 for newline character)
    offset += line.len() + 1;
  }

  tokens
}
