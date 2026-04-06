//! Symbol extraction resources.

use crate::language::Language;
use crate::symbol::SymbolMap;

use bevy_ecs::resource::Resource;
use rustc_hash::FxHashMap;

/// Function signature for symbol extraction.
pub type ExtractFn = fn(&str, u64) -> SymbolMap;

/// Resource holding registered symbol extractors per language.
///
/// Extractors are registered at startup from codelord-coder, allowing
/// codelord-core systems to call extraction without depending on
/// codelord-language.
#[derive(Resource, Default)]
pub struct SymbolExtractors {
  extractors: FxHashMap<Language, ExtractFn>,
}

impl SymbolExtractors {
  /// Create empty extractors.
  pub fn new() -> Self {
    Self {
      extractors: FxHashMap::default(),
    }
  }

  /// Register an extractor for a language.
  pub fn register(mut self, language: Language, extractor: ExtractFn) -> Self {
    self.extractors.insert(language, extractor);
    self
  }

  /// Get extractor for a language.
  pub fn get(&self, language: Language) -> Option<ExtractFn> {
    self.extractors.get(&language).copied()
  }

  /// Extract symbols using registered extractor, or return empty map.
  pub fn extract(
    &self,
    language: Language,
    source: &str,
    generation: u64,
  ) -> SymbolMap {
    self
      .extractors
      .get(&language)
      .map(|f| f(source, generation))
      .unwrap_or_else(|| SymbolMap::new(generation))
  }
}
