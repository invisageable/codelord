//! Theme system - Rendering layer (egui conversion)
//!
//! This module handles conversion from codelord-core Theme (using [u8; 4]
//! colors) to egui Visuals. Theme definitions live in codelord-core.

use codelord_core::animation::interpolate::Color;
use codelord_core::theme::components::ThemeKind;
use codelord_core::theme::resources::ThemeResource;
use codelord_core::theme::{AnimatedThemeColors, Theme, ThemeAnimation};
use codelord_core::token::TokenKind;

use eframe::egui::Color32;

/// Convert [u8; 4] to egui Color32
#[inline]
fn to_color32(rgba: [u8; 4]) -> Color32 {
  Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Convert codelord-core Color to egui Color32
#[inline]
fn color_to_color32(color: &Color) -> Color32 {
  Color32::from_rgba_unmultiplied(
    (color.r * 255.0) as u8,
    (color.g * 255.0) as u8,
    (color.b * 255.0) as u8,
    (color.a * 255.0) as u8,
  )
}

/// Convert Theme to egui Visuals
pub fn theme_to_visuals(theme: &Theme) -> eframe::egui::Visuals {
  let mut visuals = eframe::egui::Visuals::dark();

  // Widget colors
  visuals.widgets.noninteractive.bg_fill = to_color32(theme.surface0);
  visuals.widgets.noninteractive.bg_stroke.color = to_color32(theme.overlay0);
  visuals.widgets.noninteractive.fg_stroke.color = to_color32(theme.text);

  visuals.widgets.inactive.bg_fill = to_color32(theme.surface1);
  visuals.widgets.inactive.bg_stroke.color = to_color32(theme.overlay1);
  visuals.widgets.inactive.fg_stroke.color = to_color32(theme.subtext1);

  visuals.widgets.hovered.bg_fill = to_color32(theme.surface2);
  visuals.widgets.hovered.bg_stroke.color = to_color32(theme.overlay2);
  visuals.widgets.hovered.fg_stroke.color = to_color32(theme.primary);

  visuals.widgets.active.bg_fill = to_color32(theme.secondary);
  visuals.widgets.active.bg_stroke.color = to_color32(theme.button_border);
  visuals.widgets.active.fg_stroke.color = to_color32(theme.tertiary);

  visuals.widgets.open.bg_fill = to_color32(theme.surface1);
  visuals.widgets.open.bg_stroke.color = to_color32(theme.blue);
  visuals.widgets.open.fg_stroke.color = to_color32(theme.text);

  // Selection colors
  visuals.selection.bg_fill = to_color32(theme.blue).linear_multiply(0.3);
  visuals.selection.stroke.color = to_color32(theme.blue);

  visuals.hyperlink_color = to_color32(theme.blue);

  visuals.window_fill = to_color32(theme.base);
  visuals.window_stroke.color = to_color32(theme.overlay0);
  visuals.panel_fill = to_color32(theme.mantle);

  visuals.extreme_bg_color = to_color32(theme.crust);
  visuals.faint_bg_color = to_color32(theme.surface0);
  visuals.code_bg_color = to_color32(theme.mantle);

  // Separator style
  visuals.widgets.noninteractive.bg_stroke.width = 0.5;
  visuals.widgets.noninteractive.bg_stroke.color = to_color32(theme.separator);

  // Button hover colors (using weak_bg_fill fields)
  visuals.widgets.hovered.weak_bg_fill = to_color32(theme.button_hover_bg);
  visuals.widgets.active.weak_bg_fill = to_color32(theme.button_hover_fg);

  visuals
}

/// Get the current theme from the ECS World
pub fn get_theme(world: &codelord_core::ecs::world::World) -> &'static Theme {
  world
    .get_resource::<ThemeResource>()
    .map(|theme_res| match theme_res.current {
      ThemeKind::Dark => &Theme::KURO,
      ThemeKind::Light => &Theme::SHIVA,
      ThemeKind::Custom => &Theme::KURO,
    })
    .unwrap_or(&Theme::KURO)
}

/// Get animated theme visuals
///
/// If theme animation is active, returns interpolated visuals.
/// Otherwise returns static theme.
pub fn get_animated_visuals(
  world: &codelord_core::ecs::world::World,
) -> eframe::egui::Visuals {
  // Check if we have an active theme animation
  if let Some(anim) = world.get_resource::<ThemeAnimation>()
    && !anim.is_complete
  {
    // Build visuals from animated colors
    return build_visuals_from_animated(&anim.current_colors);
  }

  // No animation - use static theme
  theme_to_visuals(get_theme(world))
}

/// Build egui Visuals from animated color palette
fn build_visuals_from_animated(
  colors: &AnimatedThemeColors,
) -> eframe::egui::Visuals {
  let mut visuals = eframe::egui::Visuals::dark();

  // Convert animated colors to egui colors
  visuals.widgets.noninteractive.bg_fill = color_to_color32(&colors.surface0);
  visuals.widgets.noninteractive.bg_stroke.color =
    color_to_color32(&colors.overlay0);
  visuals.widgets.noninteractive.fg_stroke.color =
    color_to_color32(&colors.text);

  visuals.widgets.inactive.bg_fill = color_to_color32(&colors.surface1);
  visuals.widgets.inactive.bg_stroke.color = color_to_color32(&colors.overlay1);
  visuals.widgets.inactive.fg_stroke.color = color_to_color32(&colors.subtext1);

  visuals.widgets.hovered.bg_fill = color_to_color32(&colors.surface2);
  visuals.widgets.hovered.bg_stroke.color = color_to_color32(&colors.overlay2);
  visuals.widgets.hovered.fg_stroke.color = color_to_color32(&colors.primary);

  visuals.widgets.active.bg_fill = color_to_color32(&colors.secondary);
  visuals.widgets.active.bg_stroke.color =
    color_to_color32(&colors.button_border);
  visuals.widgets.active.fg_stroke.color = color_to_color32(&colors.tertiary);

  visuals.widgets.open.bg_fill = color_to_color32(&colors.surface1);
  visuals.widgets.open.bg_stroke.color = color_to_color32(&colors.blue);
  visuals.widgets.open.fg_stroke.color = color_to_color32(&colors.text);

  visuals.selection.bg_fill =
    color_to_color32(&colors.blue).linear_multiply(0.3);
  visuals.selection.stroke.color = color_to_color32(&colors.blue);

  visuals.hyperlink_color = color_to_color32(&colors.blue);

  visuals.window_fill = color_to_color32(&colors.base);
  visuals.window_stroke.color = color_to_color32(&colors.overlay0);
  visuals.panel_fill = color_to_color32(&colors.mantle);

  visuals.extreme_bg_color = color_to_color32(&colors.crust);
  visuals.faint_bg_color = color_to_color32(&colors.surface0);
  visuals.code_bg_color = color_to_color32(&colors.mantle);

  // Separator style
  visuals.widgets.noninteractive.bg_stroke.width = 0.5;
  visuals.widgets.noninteractive.bg_stroke.color =
    color_to_color32(&colors.separator);

  // Button hover colors (using weak_bg_fill fields)
  visuals.widgets.hovered.weak_bg_fill =
    color_to_color32(&colors.button_hover_bg);
  visuals.widgets.active.weak_bg_fill =
    color_to_color32(&colors.button_hover_fg);

  visuals
}

/// Map token kind to syntax highlighting color.
///
/// Colors are designed for dark theme, inspired by popular code themes.
pub fn syntax_color(kind: TokenKind) -> Color32 {
  match kind {
    // Comments - subtle gray/green
    TokenKind::Comment => Color32::from_rgb(106, 153, 85),
    TokenKind::CommentDoc => Color32::from_rgb(86, 156, 106),

    // Attributes - muted gold
    TokenKind::Attribute => Color32::from_rgb(220, 180, 100),

    // Keywords - purple/magenta
    TokenKind::Keyword => Color32::from_rgb(197, 134, 192),

    // Namespaces/modules - cyan
    TokenKind::Namespace => Color32::from_rgb(78, 201, 176),

    // Strings - orange/salmon
    TokenKind::LiteralString => Color32::from_rgb(206, 145, 120),
    TokenKind::LiteralChar => Color32::from_rgb(206, 145, 120),

    // Numbers - light green
    TokenKind::LiteralNumber => Color32::from_rgb(181, 206, 168),
    TokenKind::LiteralBool => Color32::from_rgb(86, 156, 214),

    // Identifiers - light blue for default
    TokenKind::Identifier => Color32::from_rgb(156, 220, 254),

    // Types - teal/green
    TokenKind::IdentifierType => Color32::from_rgb(78, 201, 176),

    // Functions - yellow
    TokenKind::IdentifierFunction => Color32::from_rgb(220, 220, 170),

    // Constants - blue
    TokenKind::IdentifierConstant => Color32::from_rgb(79, 193, 255),

    // Punctuation - subtle gray
    TokenKind::Punctuation => Color32::from_rgb(180, 180, 180),
    TokenKind::PunctuationBracket => Color32::from_rgb(180, 180, 180),

    // Rainbow brackets
    TokenKind::BracketLevel0 => Color32::from_rgb(255, 215, 0),
    TokenKind::BracketLevel1 => Color32::from_rgb(218, 112, 214),
    TokenKind::BracketLevel2 => Color32::from_rgb(0, 191, 255),

    // Operators - white/light
    TokenKind::Operator => Color32::from_rgb(212, 212, 212),

    // Special keywords
    TokenKind::SpecialSelf => Color32::from_rgb(86, 156, 214),
    TokenKind::SpecialMutable => Color32::from_rgb(197, 134, 192),
    TokenKind::SpecialVisibility => Color32::from_rgb(197, 134, 192),

    // Error - red
    TokenKind::Error => Color32::from_rgb(244, 71, 71),

    // Default text - white
    TokenKind::Text => Color32::from_rgb(212, 212, 212),
  }
}
