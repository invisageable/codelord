//! Color detection for source code.
//!
//! Detects color values in source code using regex patterns.
//! Supports hex, rgb, rgba, hsl, and hsla color formats.

use codelord_core::color::{ColorFormat, ColorInfo};

use regex::Regex;

use std::sync::LazyLock;

/// Regex pattern for hex colors: #RGB, #RRGGBB, #RGBA, #RRGGBBAA
/// Captures the hex digits without the # prefix.
static HEX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r#"#([0-9A-Fa-f]{3,4}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b"#).unwrap()
});

/// Regex pattern for rgb() colors.
/// Captures r, g, b values (0-255 or percentages).
static RGB_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r#"(?i)rgb\(\s*(\d{1,3}%?)\s*[,\s]\s*(\d{1,3}%?)\s*[,\s]\s*(\d{1,3}%?)\s*\)"#,
  )
  .unwrap()
});

/// Regex pattern for rgba() colors.
/// Captures r, g, b, a values.
static RGBA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r#"(?i)rgba\(\s*(\d{1,3}%?)\s*[,\s]\s*(\d{1,3}%?)\s*[,\s]\s*(\d{1,3}%?)\s*[,/\s]\s*([\d.]+%?)\s*\)"#,
  )
  .unwrap()
});

/// Regex pattern for hsl() colors.
/// Captures h, s%, l% values.
static HSL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r#"(?i)hsl\(\s*(\d{1,3}(?:\.\d+)?)\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%\s*\)"#,
  )
  .unwrap()
});

/// Regex pattern for hsla() colors.
/// Captures h, s%, l%, a values.
static HSLA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r#"(?i)hsla\(\s*(\d{1,3}(?:\.\d+)?)\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%\s*[,/\s]\s*([\d.]+%?)\s*\)"#,
  )
  .unwrap()
});

/// Extracts all color values from source text.
///
/// Returns a vector of `ColorInfo` with position and parsed RGBA values.
pub fn extract(source: &str) -> Vec<ColorInfo> {
  let mut colors = Vec::new();

  // Build line index for efficient line/column calculation
  let line_starts: Vec<usize> = std::iter::once(0)
    .chain(source.match_indices('\n').map(|(i, _)| i + 1))
    .collect();

  // Extract hex colors
  for cap in HEX_PATTERN.captures_iter(source) {
    let m = cap.get(0).unwrap();
    let hex = cap.get(1).unwrap().as_str();

    if let Some(rgba) = parse_hex(hex) {
      let (line, column) = byte_to_line_col(&line_starts, m.start());

      colors.push(ColorInfo {
        start: m.start(),
        end: m.end(),
        line,
        column,
        rgba,
        text: m.as_str().to_string(),
        format: ColorFormat::Hex,
      });
    }
  }

  // Extract rgb() colors
  for cap in RGB_PATTERN.captures_iter(source) {
    let m = cap.get(0).unwrap();

    if let Some(rgba) = parse_rgb(&cap) {
      let (line, column) = byte_to_line_col(&line_starts, m.start());

      colors.push(ColorInfo {
        start: m.start(),
        end: m.end(),
        line,
        column,
        rgba,
        text: m.as_str().to_string(),
        format: ColorFormat::Rgb,
      });
    }
  }

  // Extract rgba() colors
  for cap in RGBA_PATTERN.captures_iter(source) {
    let m = cap.get(0).unwrap();

    if let Some(rgba) = parse_rgba(&cap) {
      let (line, column) = byte_to_line_col(&line_starts, m.start());

      colors.push(ColorInfo {
        start: m.start(),
        end: m.end(),
        line,
        column,
        rgba,
        text: m.as_str().to_string(),
        format: ColorFormat::Rgba,
      });
    }
  }

  // Extract hsl() colors
  for cap in HSL_PATTERN.captures_iter(source) {
    let m = cap.get(0).unwrap();

    if let Some(rgba) = parse_hsl(&cap) {
      let (line, column) = byte_to_line_col(&line_starts, m.start());

      colors.push(ColorInfo {
        start: m.start(),
        end: m.end(),
        line,
        column,
        rgba,
        text: m.as_str().to_string(),
        format: ColorFormat::Hsl,
      });
    }
  }

  // Extract hsla() colors
  for cap in HSLA_PATTERN.captures_iter(source) {
    let m = cap.get(0).unwrap();

    if let Some(rgba) = parse_hsla(&cap) {
      let (line, column) = byte_to_line_col(&line_starts, m.start());

      colors.push(ColorInfo {
        start: m.start(),
        end: m.end(),
        line,
        column,
        rgba,
        text: m.as_str().to_string(),
        format: ColorFormat::Hsla,
      });
    }
  }

  // Sort by position
  colors.sort_by_key(|c| c.start);
  colors
}

/// Converts byte offset to (line, column) tuple.
fn byte_to_line_col(
  line_starts: &[usize],
  byte_offset: usize,
) -> (usize, usize) {
  let line = line_starts
    .iter()
    .rposition(|&start| start <= byte_offset)
    .unwrap_or(0);

  let column = byte_offset - line_starts[line];
  (line, column)
}

/// Parses hex color string to RGBA.
fn parse_hex(hex: &str) -> Option<[u8; 4]> {
  match hex.len() {
    // #RGB -> #RRGGBB
    3 => {
      let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
      let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
      let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
      Some([r, g, b, 255])
    }
    // #RGBA -> #RRGGBBAA
    4 => {
      let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
      let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
      let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
      let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
      Some([r, g, b, a])
    }
    // #RRGGBB
    6 => {
      let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
      let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
      let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
      Some([r, g, b, 255])
    }
    // #RRGGBBAA
    8 => {
      let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
      let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
      let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
      let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
      Some([r, g, b, a])
    }
    _ => None,
  }
}

/// Parses rgb() capture to RGBA.
fn parse_rgb(cap: &regex::Captures) -> Option<[u8; 4]> {
  let r = parse_color_value(cap.get(1)?.as_str())?;
  let g = parse_color_value(cap.get(2)?.as_str())?;
  let b = parse_color_value(cap.get(3)?.as_str())?;
  Some([r, g, b, 255])
}

/// Parses rgba() capture to RGBA.
fn parse_rgba(cap: &regex::Captures) -> Option<[u8; 4]> {
  let r = parse_color_value(cap.get(1)?.as_str())?;
  let g = parse_color_value(cap.get(2)?.as_str())?;
  let b = parse_color_value(cap.get(3)?.as_str())?;
  let a = parse_alpha_value(cap.get(4)?.as_str())?;
  Some([r, g, b, a])
}

/// Parses hsl() capture to RGBA.
fn parse_hsl(cap: &regex::Captures) -> Option<[u8; 4]> {
  let h: f32 = cap.get(1)?.as_str().parse().ok()?;
  let s: f32 = cap.get(2)?.as_str().parse().ok()?;
  let l: f32 = cap.get(3)?.as_str().parse().ok()?;

  let (r, g, b) = hsl_to_rgb(h, s / 100.0, l / 100.0);
  Some([r, g, b, 255])
}

/// Parses hsla() capture to RGBA.
fn parse_hsla(cap: &regex::Captures) -> Option<[u8; 4]> {
  let h: f32 = cap.get(1)?.as_str().parse().ok()?;
  let s: f32 = cap.get(2)?.as_str().parse().ok()?;
  let l: f32 = cap.get(3)?.as_str().parse().ok()?;
  let a = parse_alpha_value(cap.get(4)?.as_str())?;

  let (r, g, b) = hsl_to_rgb(h, s / 100.0, l / 100.0);
  Some([r, g, b, a])
}

/// Parses a color value (0-255 or percentage).
fn parse_color_value(s: &str) -> Option<u8> {
  if s.ends_with('%') {
    let pct: f32 = s.trim_end_matches('%').parse().ok()?;
    Some((pct * 2.55).round() as u8)
  } else {
    s.parse().ok()
  }
}

/// Parses an alpha value (0-1 or percentage).
fn parse_alpha_value(s: &str) -> Option<u8> {
  if s.ends_with('%') {
    let pct: f32 = s.trim_end_matches('%').parse().ok()?;
    Some((pct * 2.55).round() as u8)
  } else {
    let a: f32 = s.parse().ok()?;
    Some((a * 255.0).round() as u8)
  }
}

/// Converts HSL to RGB.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
  let h = h % 360.0;

  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
  let m = l - c / 2.0;

  let (r, g, b) = match h as u32 {
    0..=59 => (c, x, 0.0),
    60..=119 => (x, c, 0.0),
    120..=179 => (0.0, c, x),
    180..=239 => (0.0, x, c),
    240..=299 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };

  (
    ((r + m) * 255.0).round() as u8,
    ((g + m) * 255.0).round() as u8,
    ((b + m) * 255.0).round() as u8,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_hex_colors() {
    let source =
      "let a = \"#fff\"; let b = \"#FF0000\"; let c = \"#00ff0080\";";
    let colors = extract(source);

    assert_eq!(colors.len(), 3);

    // #fff -> white
    assert_eq!(colors[0].rgba, [255, 255, 255, 255]);

    // #FF0000 -> red
    assert_eq!(colors[1].rgba, [255, 0, 0, 255]);

    // #00ff0080 -> green with 50% alpha
    assert_eq!(colors[2].rgba, [0, 255, 0, 128]);
  }

  #[test]
  fn test_rgb_colors() {
    let source = "color: rgb(204, 253, 62);";
    let colors = extract(source);

    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0].rgba, [204, 253, 62, 255]);
  }

  #[test]
  fn test_rgba_colors() {
    let source = "color: rgba(255, 0, 0, 0.5);";
    let colors = extract(source);

    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0].rgba, [255, 0, 0, 128]);
  }

  #[test]
  fn test_hsl_colors() {
    let source = "color: hsl(0, 100%, 50%);";
    let colors = extract(source);

    assert_eq!(colors.len(), 1);
    // Pure red
    assert_eq!(colors[0].rgba, [255, 0, 0, 255]);
  }
}
