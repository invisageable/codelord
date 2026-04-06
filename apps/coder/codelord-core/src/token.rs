use crate::language::Language;

use bevy_ecs::resource::Resource;
use rustc_hash::FxHashMap;

/// Function signature for token extraction (syntax highlighting).
pub type ExtractTokensFn = fn(&str) -> Vec<Token>;

/// Resource holding registered token extractors per language.
///
/// Extractors are registered at startup from codelord-coder, allowing
/// components to call extraction without depending on codelord-language.
#[derive(Resource, Default)]
pub struct TokenExtractors {
  extractors: FxHashMap<Language, ExtractTokensFn>,
}

impl TokenExtractors {
  /// Create empty extractors.
  pub fn new() -> Self {
    Self {
      extractors: FxHashMap::default(),
    }
  }

  /// Register an extractor for a language.
  pub fn register(
    mut self,
    language: Language,
    extractor: ExtractTokensFn,
  ) -> Self {
    self.extractors.insert(language, extractor);
    self
  }

  /// Get extractor for a language.
  pub fn get(&self, language: Language) -> Option<ExtractTokensFn> {
    self.extractors.get(&language).copied()
  }

  /// Extract tokens using registered extractor, or return empty vec.
  pub fn extract(&self, language: Language, source: &str) -> Vec<Token> {
    self
      .extractors
      .get(&language)
      .map(|f| f(source))
      .unwrap_or_default()
  }
}

/// A token with position information.
#[derive(Debug, Clone)]
pub struct Token {
  pub kind: TokenKind,
  pub start: usize,
  pub end: usize,
}

/// Token classification for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
  /// A comment.
  Comment,
  /// A documentation comment.
  CommentDoc,
  /// Attributes (e.g., #[derive], #[cfg]).
  Attribute,
  /// A keyword.
  Keyword,
  /// A namespace (crate names, module paths).
  Namespace,
  /// A literal string.
  LiteralString,
  /// A literal character.
  LiteralChar,
  /// A literal number.
  LiteralNumber,
  /// A literal boolean.
  LiteralBool,
  /// An identifier.
  Identifier,
  /// Type names (capitalized).
  IdentifierType,
  /// Function names.
  IdentifierFunction,
  /// Constant names.
  IdentifierConstant,
  /// Punctuation.
  Punctuation,
  /// Bracket punctuation.
  PunctuationBracket,
  /// Rainbow bracket level 0.
  BracketLevel0,
  /// Rainbow bracket level 1.
  BracketLevel1,
  /// Rainbow bracket level 2.
  BracketLevel2,
  /// Operators.
  Operator,
  /// `self` keyword.
  SpecialSelf,
  /// `mut` keyword.
  SpecialMutable,
  /// `pub` keyword.
  SpecialVisibility,
  /// Error tokens.
  Error,
  /// Default text.
  Text,
}
