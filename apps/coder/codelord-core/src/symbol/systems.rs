//! Symbol extraction systems.

use crate::language::Language;
use crate::symbol::TabSymbols;
use crate::symbol::resources::SymbolExtractors;
use crate::tabbar::components::EditorTab;
use crate::text_editor::components::{FileTab, TextBuffer};

use bevy_ecs::query::With;
use bevy_ecs::system::{Query, Res};

/// System: extracts symbols from dirty TabSymbols.
///
/// Uses registered extractors from SymbolExtractors resource.
pub fn extract_symbols_system(
  extractors: Res<SymbolExtractors>,
  mut tabs: Query<
    (&TextBuffer, Option<&FileTab>, &mut TabSymbols),
    With<EditorTab>,
  >,
) {
  for (buffer, file_tab, mut symbols) in tabs.iter_mut() {
    if !symbols.dirty {
      continue;
    }

    let language = file_tab
      .map(|ft| {
        // First try the standard extension
        if let Some(ext) = ft.path.extension().and_then(|e| e.to_str()) {
          return Language::from(ext);
        }

        // Handle dotfiles like .env, .gitignore, etc.
        if let Some(name) = ft.path.file_name().and_then(|n| n.to_str())
          && let Some(stripped) = name.strip_prefix('.')
        {
          return Language::from(stripped);
        }

        Language::default()
      })
      .unwrap_or_default();

    let source = buffer.to_string();
    let generation = symbols.map.generation.wrapping_add(1);
    let new_map = extractors.extract(language, &source, generation);

    symbols.update(new_map);
  }
}
