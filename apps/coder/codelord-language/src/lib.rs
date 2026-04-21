pub mod color;
pub mod language;

use codelord_core::ecs::world::World;

/// Register per-language symbol extractors into the world. Symbol
/// extraction uses tree-sitter to collect function/type declarations
/// for breadcrumbs and go-to-symbol.
pub fn install_symbol_extractors(world: &mut World) {
  use codelord_core::language::Language;
  use codelord_core::symbol::resources::SymbolExtractors;

  world.insert_resource(
    SymbolExtractors::new()
      .register(Language::C, language::c::symbols::extract_symbols)
      .register(Language::Elixir, language::elixir::symbols::extract_symbols)
      .register(
        Language::JavaScript,
        language::javascript::symbols::extract_symbols,
      )
      .register(Language::Json, language::json::symbols::extract_symbols)
      .register(Language::Python, language::python::symbols::extract_symbols)
      .register(Language::Rust, language::rust::symbols::extract_symbols)
      .register(Language::Zig, language::zig::symbols::extract_symbols)
      .register(Language::Zo, language::zo::symbols::extract_symbols),
  );
}

/// Register per-language token extractors (syntax-highlighting parsers)
/// into the world.
pub fn install_token_extractors(world: &mut World) {
  use codelord_core::language::Language;
  use codelord_core::token::TokenExtractors;

  world.insert_resource(
    TokenExtractors::new()
      .register(Language::Bash, language::bash::highlights::parse)
      .register(Language::C, language::c::highlights::parse)
      .register(Language::Conf, language::conf::highlights::parse)
      .register(Language::Css, language::css::highlights::parse)
      .register(Language::Diff, language::diff::highlights::parse)
      .register(Language::Elixir, language::elixir::highlights::parse)
      .register(Language::Env, language::env::highlights::parse)
      .register(Language::Gleam, language::gleam::highlights::parse)
      .register(Language::Go, language::go::highlights::parse)
      .register(Language::Html, language::html::highlights::parse)
      .register(
        Language::JavaScript,
        language::javascript::highlights::parse,
      )
      .register(Language::Json, language::json::highlights::parse)
      .register(Language::Markdown, language::markdown::highlights::parse)
      .register(Language::Python, language::python::highlights::parse)
      .register(Language::Rust, language::rust::highlights::parse)
      .register(
        Language::TypeScript,
        language::typescript::highlights::parse,
      )
      .register(Language::Yaml, language::yaml::highlights::parse)
      .register(Language::Zig, language::zig::highlights::parse)
      .register(Language::Zo, language::zo::highlights::parse),
  );
}
