//! # font.
//!
//! > *font configuration for the code editor*.
//!
//! This module handles font setup to ensure proper text rendering,
//! especially for monospace fonts in the code editor.
//!
//! `firacode` is used as monospace for code in editor, numbers in metrics
//! or code block for markdown rendering.
//!
//! `roboto-flex` is used an alternative for `firacode`. it should be remove
//! for production.
//!
//! `suisse-intl` is used for content.
//!
//! combo fonts are use for special title splits by syllabes.
//! fonts are aeonik and cirka like for `reachout`:
//! - `reach` (aeonik).
//! - `out` (cirka).

use eframe::egui;

use std::sync::Arc;

/// The `aeonik` font name.
pub const AEONIK: &str = "aeonik";
/// The `cirka` font name.
pub const CIRKA: &str = "cirka";
/// The `firacode` font name.
pub const FIRACODE: &str = "firacode";
/// The `roboto-flex` font name.
pub const ROBOTO_FLEX: &str = "roboto-flex";
/// The `suisse-intl` font name.
pub const SUISSE_INTL: &str = "suisse-intl";

/// Sized `aeonik` FontId.
#[inline]
pub fn aeonik(size: f32) -> egui::FontId {
  egui::FontId::new(size, egui::FontFamily::Name(AEONIK.into()))
}

/// Sized `cirka` FontId.
#[inline]
pub fn cirka(size: f32) -> egui::FontId {
  egui::FontId::new(size, egui::FontFamily::Name(CIRKA.into()))
}

/// Sized `firacode` FontId.
#[inline]
pub fn firacode(size: f32) -> egui::FontId {
  egui::FontId::new(size, egui::FontFamily::Name(FIRACODE.into()))
}

/// Sized `suisse-intl` FontId.
#[inline]
pub fn suisse(size: f32) -> egui::FontId {
  egui::FontId::new(size, egui::FontFamily::Name(SUISSE_INTL.into()))
}

/// Adds new fonts here.
fn fonts() -> &'static Vec<Font> {
  static FONTS: std::sync::OnceLock<Vec<Font>> = std::sync::OnceLock::new();

  FONTS.get_or_init(|| {
    Vec::from([
      Font::new(
        AEONIK,
        include_bytes!("../../../codelord-assets/font/font-aeonik-regular.ttf"),
      ),
      Font::new(
        CIRKA,
        include_bytes!("../../../codelord-assets/font/font-cirka-regular.ttf"),
      ),
      Font::new(
        FIRACODE,
        include_bytes!(
          "../../../codelord-assets/font/font-firacode-variable.ttf"
        ),
      ),
      Font::new(
        ROBOTO_FLEX,
        include_bytes!("../../../codelord-assets/font/font-roboto-flex.ttf"),
      ),
      Font::new(
        SUISSE_INTL,
        include_bytes!(
          "../../../codelord-assets/font/font-suisse-intl-regular.otf"
        ),
      ),
    ])
  })
}

/// Represents a [`Font`] instance — A kind of wrapper for [`egui::FontData`].
#[derive(Clone, Debug)]
pub struct Font {
  /// The font name.
  name: &'static str,
  /// The font data.
  data: egui::FontData,
}

impl Font {
  /// Creates a new [`Font`] instance.
  #[inline(always)]
  pub fn new(name: &'static str, bytes: &'static [u8]) -> Self {
    Self {
      name,
      data: egui::FontData::from_static(bytes),
    }
  }
}

/// Returns the base font definitions with all app fonts.
pub fn base_font_definitions() -> egui::FontDefinitions {
  let mut font_definitions = egui::FontDefinitions::default();

  font_definitions
    .families
    .entry(egui::FontFamily::Monospace)
    .or_default()
    .clear();

  for font in fonts().iter() {
    font_definitions
      .font_data
      .insert(font.name.into(), Arc::new(font.data.to_owned()));

    font_definitions.families.insert(
      egui::FontFamily::Name(font.name.into()),
      vec![font.name.into()],
    );
  }

  font_definitions
    .families
    .entry(egui::FontFamily::Monospace)
    .or_default()
    .insert(0, fonts()[2].name.into());

  font_definitions
    .families
    .entry(egui::FontFamily::Proportional)
    .or_default()
    .insert(0, fonts()[4].name.into());

  font_definitions
}

/// Installs all the available fonts.
pub fn install_fonts(ctx: &egui::Context) {
  ctx.set_fonts(base_font_definitions());

  ctx.global_style_mut(|style| {
    style.text_styles.insert(
      egui::TextStyle::Monospace,
      egui::FontId::new(12.0, egui::FontFamily::Monospace),
    );
  });
}
