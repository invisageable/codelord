//! Color detection and preview support.
//!
//! Provides types for detecting and caching color values in source code.
//! Colors are detected via regex patterns and cached per-file for rendering.

use bevy_ecs::resource::Resource;
use rustc_hash::FxHashMap as HashMap;

use std::path::PathBuf;

/// Detected color information with position and parsed value.
#[derive(Debug, Clone)]
pub struct ColorInfo {
  /// Byte offset where the color text starts.
  pub start: usize,
  /// Byte offset where the color text ends.
  pub end: usize,
  /// Line number (0-indexed) where the color appears.
  pub line: usize,
  /// Column offset (0-indexed) within the line.
  pub column: usize,
  /// Parsed RGBA color value (0-255 per channel).
  pub rgba: [u8; 4],
  /// Original color text (e.g., "#ff0000", "rgb(255, 0, 0)").
  pub text: String,
  /// Color format that was detected.
  pub format: ColorFormat,
}

/// Supported color format types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorFormat {
  /// Hex color: #RGB, #RRGGBB, #RGBA, #RRGGBBAA
  #[default]
  Hex,
  /// RGB function: rgb(r, g, b)
  Rgb,
  /// RGBA function: rgba(r, g, b, a)
  Rgba,
  /// HSL function: hsl(h, s%, l%)
  Hsl,
  /// HSLA function: hsla(h, s%, l%, a)
  Hsla,
}

/// Cache of detected colors per file.
///
/// Colors are re-detected when file content changes (tracked by generation).
#[derive(Resource, Default)]
pub struct ColorCache {
  /// Map from file path to detected colors and generation.
  entries: HashMap<PathBuf, ColorCacheEntry>,
}

/// Cache entry for a single file.
#[derive(Default, Clone)]
pub struct ColorCacheEntry {
  /// Generation counter to detect stale cache.
  pub generation: u64,
  /// Detected colors in this file.
  pub colors: Vec<ColorInfo>,
}

impl ColorCache {
  /// Creates a new empty color cache.
  pub fn new() -> Self {
    Self::default()
  }

  /// Gets cached colors for a file if the generation matches.
  pub fn get(&self, path: &PathBuf, generation: u64) -> Option<&[ColorInfo]> {
    self.entries.get(path).and_then(|entry| {
      if entry.generation == generation {
        Some(entry.colors.as_slice())
      } else {
        None
      }
    })
  }

  /// Updates the cache for a file.
  pub fn update(
    &mut self,
    path: PathBuf,
    generation: u64,
    colors: Vec<ColorInfo>,
  ) {
    self
      .entries
      .insert(path, ColorCacheEntry { generation, colors });
  }

  /// Removes a file from the cache.
  pub fn remove(&mut self, path: &PathBuf) {
    self.entries.remove(path);
  }

  /// Clears the entire cache.
  pub fn clear(&mut self) {
    self.entries.clear();
  }

  /// Gets colors for a specific line (for efficient rendering).
  pub fn colors_for_line(
    &self,
    path: &PathBuf,
    line: usize,
  ) -> Vec<&ColorInfo> {
    self
      .entries
      .get(path)
      .map(|entry| entry.colors.iter().filter(|c| c.line == line).collect())
      .unwrap_or_default()
  }
}

/// Function signature for color extraction.
pub type ExtractColorsFn = fn(&str) -> Vec<ColorInfo>;

/// Resource holding the color extractor function.
///
/// Registered at startup from codelord-coder, allowing components to
/// extract colors without depending on codelord-language.
#[derive(Resource)]
pub struct ColorExtractor {
  extractor: ExtractColorsFn,
}

impl ColorExtractor {
  /// Creates a new color extractor with the given function.
  pub fn new(extractor: ExtractColorsFn) -> Self {
    Self { extractor }
  }

  /// Extracts colors from source text.
  pub fn extract(&self, source: &str) -> Vec<ColorInfo> {
    (self.extractor)(source)
  }
}

impl Default for ColorExtractor {
  fn default() -> Self {
    Self {
      extractor: |_| Vec::new(),
    }
  }
}

/// State for the color picker popup.
#[derive(Resource, Default, Clone)]
pub struct ColorPickerState {
  /// Whether the picker is currently open.
  pub open: bool,
  /// The entity (tab) containing the color being edited.
  pub entity: Option<bevy_ecs::entity::Entity>,
  /// Byte range of the color text in the buffer.
  pub byte_range: Option<(usize, usize)>,
  /// Line index where the color is located.
  pub line: usize,
  /// Current color value being edited (RGBA).
  pub color: [f32; 4],
  /// Original color format for replacement.
  pub format: ColorFormat,
  /// Screen position for the popup.
  pub position: (f32, f32),
}

impl ColorPickerState {
  /// Opens the color picker for editing a color.
  pub fn open(
    &mut self,
    entity: bevy_ecs::entity::Entity,
    color_info: &ColorInfo,
    screen_pos: (f32, f32),
  ) {
    self.open = true;
    self.entity = Some(entity);
    self.byte_range = Some((color_info.start, color_info.end));
    self.line = color_info.line;
    self.format = color_info.format;
    self.position = screen_pos;

    // Convert u8 RGBA to f32 for egui color picker.
    self.color = [
      color_info.rgba[0] as f32 / 255.0,
      color_info.rgba[1] as f32 / 255.0,
      color_info.rgba[2] as f32 / 255.0,
      color_info.rgba[3] as f32 / 255.0,
    ];
  }

  /// Closes the color picker.
  pub fn close(&mut self) {
    self.open = false;
    self.entity = None;
    self.byte_range = None;
  }

  /// Formats the current color as a string in the original format.
  pub fn format_color(&self) -> String {
    let r = (self.color[0] * 255.0).round() as u8;
    let g = (self.color[1] * 255.0).round() as u8;
    let b = (self.color[2] * 255.0).round() as u8;
    let a = (self.color[3] * 255.0).round() as u8;

    match self.format {
      ColorFormat::Hex => {
        if a == 255 {
          format!("#{:02x}{:02x}{:02x}", r, g, b)
        } else {
          format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        }
      }
      ColorFormat::Rgb => {
        format!("rgb({}, {}, {})", r, g, b)
      }
      ColorFormat::Rgba => {
        let alpha = self.color[3];
        format!("rgba({}, {}, {}, {:.2})", r, g, b, alpha)
      }
      ColorFormat::Hsl => {
        let (h, s, l) = rgb_to_hsl(r, g, b);
        format!(
          "hsl({}, {}%, {}%)",
          h.round() as i32,
          (s * 100.0).round() as i32,
          (l * 100.0).round() as i32
        )
      }
      ColorFormat::Hsla => {
        let (h, s, l) = rgb_to_hsl(r, g, b);
        let alpha = self.color[3];
        format!(
          "hsla({}, {}%, {}%, {:.2})",
          h.round() as i32,
          (s * 100.0).round() as i32,
          (l * 100.0).round() as i32,
          alpha
        )
      }
    }
  }
}

/// Converts RGB (0-255) to HSL.
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = r as f32 / 255.0;
  let g = g as f32 / 255.0;
  let b = b as f32 / 255.0;

  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let l = (max + min) / 2.0;

  if (max - min).abs() < f32::EPSILON {
    return (0.0, 0.0, l);
  }

  let d = max - min;
  let s = if l > 0.5 {
    d / (2.0 - max - min)
  } else {
    d / (max + min)
  };

  let h = if (max - r).abs() < f32::EPSILON {
    ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
  } else if (max - g).abs() < f32::EPSILON {
    ((b - r) / d + 2.0) / 6.0
  } else {
    ((r - g) / d + 4.0) / 6.0
  };

  (h * 360.0, s, l)
}
