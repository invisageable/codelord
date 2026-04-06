//! Configuration file (.conf) syntax highlighting.
//!
//! Reuses the env tokenizer since .conf files have the same format:
//! - Lines starting with `#` are comments
//! - `KEY=VALUE` pairs

pub use crate::language::env::highlights::parse;
