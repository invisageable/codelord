//! Theme color definitions
//!
//! Colors defined as [u8; 4] arrays (RGBA) to avoid egui dependency.

pub mod components;
pub mod resources;
pub mod systems;

use crate::animation::interpolate::{Color, Interpolate};

use bevy_ecs::resource::Resource;
use eazy::Curve;
use eazy::interpolation::Interpolation;

/// Complete color palette for IDE theming
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Theme {
  // Accent colors
  pub rosewater: [u8; 4],
  pub flamingo: [u8; 4],
  pub pink: [u8; 4],
  pub mauve: [u8; 4],
  pub red: [u8; 4],
  pub maroon: [u8; 4],
  pub peach: [u8; 4],
  pub yellow: [u8; 4],
  pub green: [u8; 4],
  pub teal: [u8; 4],
  pub sky: [u8; 4],
  pub sapphire: [u8; 4],
  pub blue: [u8; 4],
  pub lavender: [u8; 4],

  // Text colors
  pub text: [u8; 4],
  pub subtext1: [u8; 4],
  pub subtext0: [u8; 4],

  // Overlay colors
  pub overlay2: [u8; 4],
  pub overlay1: [u8; 4],
  pub overlay0: [u8; 4],

  // Surface colors
  pub surface2: [u8; 4],
  pub surface1: [u8; 4],
  pub surface0: [u8; 4],

  // Base colors
  pub base: [u8; 4],
  pub mantle: [u8; 4],
  pub crust: [u8; 4],

  // Brand colors
  pub primary: [u8; 4],
  pub secondary: [u8; 4],
  pub tertiary: [u8; 4],

  // UI element colors
  pub separator: [u8; 4],
  pub button_border: [u8; 4],

  // Button hover colors
  pub button_hover_bg: [u8; 4],
  pub button_hover_fg: [u8; 4],
}

impl Theme {
  /// KURO theme - Dark theme with pure black background
  pub const KURO: Self = Self {
    rosewater: [245, 224, 220, 255],
    flamingo: [242, 205, 205, 255],
    pink: [245, 194, 231, 255],
    mauve: [203, 166, 247, 255],
    red: [243, 139, 168, 255],
    maroon: [235, 160, 172, 255],
    peach: [250, 179, 135, 255],
    yellow: [249, 226, 175, 255],
    green: [166, 227, 161, 255],
    teal: [148, 226, 213, 255],
    sky: [137, 220, 235, 255],
    sapphire: [116, 199, 236, 255],
    blue: [137, 180, 250, 255],
    lavender: [180, 190, 254, 255],
    text: [205, 214, 244, 255],
    subtext1: [186, 194, 222, 255],
    subtext0: [166, 173, 200, 255],
    overlay2: [147, 153, 178, 255],
    overlay1: [127, 132, 156, 255],
    overlay0: [108, 112, 134, 255],
    surface2: [88, 91, 112, 255],
    surface1: [69, 71, 90, 255],
    surface0: [0, 0, 0, 255], // Pure black gutter background
    base: [0, 0, 0, 255],     // Pure black
    mantle: [10, 10, 10, 255], // Near black
    crust: [0, 0, 0, 255],    // Pure black for editor
    primary: [204, 253, 62, 255], // Lime green
    secondary: [0, 0, 0, 255], // Black
    tertiary: [255, 255, 255, 255], // White
    separator: [30, 30, 30, 255],
    button_border: [64, 64, 64, 255], // Gray for button border on hover
    button_hover_bg: [255, 255, 255, 255], // White
    button_hover_fg: [0, 0, 0, 255],  // Black
  };

  /// SHIVA theme - Light theme with pure white background
  pub const SHIVA: Self = Self {
    rosewater: [220, 138, 120, 255],
    flamingo: [221, 120, 120, 255],
    pink: [234, 118, 203, 255],
    mauve: [136, 57, 239, 255],
    red: [210, 15, 57, 255],
    maroon: [230, 69, 83, 255],
    peach: [254, 100, 11, 255],
    yellow: [223, 142, 29, 255],
    green: [64, 160, 43, 255],
    teal: [23, 146, 153, 255],
    sky: [4, 165, 229, 255],
    sapphire: [32, 159, 181, 255],
    blue: [30, 102, 245, 255],
    lavender: [114, 135, 253, 255],
    text: [76, 79, 105, 255],
    subtext1: [92, 95, 119, 255],
    subtext0: [108, 111, 133, 255],
    overlay2: [124, 127, 147, 255],
    overlay1: [140, 143, 161, 255],
    overlay0: [156, 160, 176, 255],
    surface2: [172, 176, 190, 255],
    surface1: [188, 192, 204, 255],
    surface0: [255, 255, 255, 255], // Pure white gutter background
    base: [255, 255, 255, 255],     // Pure white
    mantle: [245, 245, 245, 255],   // Near white
    crust: [255, 255, 255, 255],    // Pure white for editor
    primary: [0, 0, 0, 255],        // Black
    secondary: [255, 255, 255, 255], // White (same as base)
    tertiary: [255, 255, 255, 255], // White
    separator: [200, 200, 200, 255], // Light gray separator
    button_border: [64, 64, 64, 255], // Gray for button border on hover
    button_hover_bg: [0, 0, 0, 255], // Black
    button_hover_fg: [255, 255, 255, 255], // White
  };
}

// ============================================================================
// Theme Animation
// ============================================================================

/// Animated theme colors
///
/// Holds all theme colors being animated during theme transition.
/// Uses codelord-core Color type (no egui dependency).
#[derive(Clone)]
pub struct AnimatedThemeColors {
  pub base: Color,
  pub mantle: Color,
  pub crust: Color,
  pub text: Color,
  pub subtext0: Color,
  pub subtext1: Color,
  pub surface0: Color,
  pub surface1: Color,
  pub surface2: Color,
  pub overlay0: Color,
  pub overlay1: Color,
  pub overlay2: Color,
  pub rosewater: Color,
  pub flamingo: Color,
  pub pink: Color,
  pub mauve: Color,
  pub red: Color,
  pub maroon: Color,
  pub peach: Color,
  pub yellow: Color,
  pub green: Color,
  pub teal: Color,
  pub sky: Color,
  pub sapphire: Color,
  pub blue: Color,
  pub lavender: Color,
  pub primary: Color,
  pub secondary: Color,
  pub tertiary: Color,
  pub separator: Color,
  pub button_border: Color,
  pub button_hover_bg: Color,
  pub button_hover_fg: Color,
}

impl AnimatedThemeColors {
  /// Create from Theme
  pub fn from_theme(theme: &Theme) -> Self {
    fn to_color(rgba: [u8; 4]) -> Color {
      Color {
        r: rgba[0] as f32 / 255.0,
        g: rgba[1] as f32 / 255.0,
        b: rgba[2] as f32 / 255.0,
        a: rgba[3] as f32 / 255.0,
      }
    }

    Self {
      base: to_color(theme.base),
      mantle: to_color(theme.mantle),
      crust: to_color(theme.crust),
      text: to_color(theme.text),
      subtext0: to_color(theme.subtext0),
      subtext1: to_color(theme.subtext1),
      surface0: to_color(theme.surface0),
      surface1: to_color(theme.surface1),
      surface2: to_color(theme.surface2),
      overlay0: to_color(theme.overlay0),
      overlay1: to_color(theme.overlay1),
      overlay2: to_color(theme.overlay2),
      rosewater: to_color(theme.rosewater),
      flamingo: to_color(theme.flamingo),
      pink: to_color(theme.pink),
      mauve: to_color(theme.mauve),
      red: to_color(theme.red),
      maroon: to_color(theme.maroon),
      peach: to_color(theme.peach),
      yellow: to_color(theme.yellow),
      green: to_color(theme.green),
      teal: to_color(theme.teal),
      sky: to_color(theme.sky),
      sapphire: to_color(theme.sapphire),
      blue: to_color(theme.blue),
      lavender: to_color(theme.lavender),
      primary: to_color(theme.primary),
      secondary: to_color(theme.secondary),
      tertiary: to_color(theme.tertiary),
      separator: to_color(theme.separator),
      button_border: to_color(theme.button_border),
      button_hover_bg: to_color(theme.button_hover_bg),
      button_hover_fg: to_color(theme.button_hover_fg),
    }
  }
}

impl Interpolate for AnimatedThemeColors {
  fn lerp(&self, target: &Self, t: f32) -> Self {
    Self {
      base: self.base.lerp(&target.base, t),
      mantle: self.mantle.lerp(&target.mantle, t),
      crust: self.crust.lerp(&target.crust, t),
      text: self.text.lerp(&target.text, t),
      subtext0: self.subtext0.lerp(&target.subtext0, t),
      subtext1: self.subtext1.lerp(&target.subtext1, t),
      surface0: self.surface0.lerp(&target.surface0, t),
      surface1: self.surface1.lerp(&target.surface1, t),
      surface2: self.surface2.lerp(&target.surface2, t),
      overlay0: self.overlay0.lerp(&target.overlay0, t),
      overlay1: self.overlay1.lerp(&target.overlay1, t),
      overlay2: self.overlay2.lerp(&target.overlay2, t),
      rosewater: self.rosewater.lerp(&target.rosewater, t),
      flamingo: self.flamingo.lerp(&target.flamingo, t),
      pink: self.pink.lerp(&target.pink, t),
      mauve: self.mauve.lerp(&target.mauve, t),
      red: self.red.lerp(&target.red, t),
      maroon: self.maroon.lerp(&target.maroon, t),
      peach: self.peach.lerp(&target.peach, t),
      yellow: self.yellow.lerp(&target.yellow, t),
      green: self.green.lerp(&target.green, t),
      teal: self.teal.lerp(&target.teal, t),
      sky: self.sky.lerp(&target.sky, t),
      sapphire: self.sapphire.lerp(&target.sapphire, t),
      blue: self.blue.lerp(&target.blue, t),
      lavender: self.lavender.lerp(&target.lavender, t),
      primary: self.primary.lerp(&target.primary, t),
      secondary: self.secondary.lerp(&target.secondary, t),
      tertiary: self.tertiary.lerp(&target.tertiary, t),
      separator: self.separator.lerp(&target.separator, t),
      button_border: self.button_border.lerp(&target.button_border, t),
      button_hover_bg: self.button_hover_bg.lerp(&target.button_hover_bg, t),
      button_hover_fg: self.button_hover_fg.lerp(&target.button_hover_fg, t),
    }
  }

  fn is_close(&self, other: &Self, epsilon: f32) -> bool {
    self.base.is_close(&other.base, epsilon)
      && self.mantle.is_close(&other.mantle, epsilon)
      && self.text.is_close(&other.text, epsilon)
  }
}

/// Insert theme resources + message queues into the world.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::ecs::message::Messages;
  use crate::theme::components::ThemeKind;
  use crate::theme::resources::{
    ThemeChangedEvent, ThemeCommand, ThemeResource,
  };

  world.insert_resource(ThemeResource::new(ThemeKind::Dark));
  world.init_resource::<Messages<ThemeCommand>>();
  world.init_resource::<Messages<ThemeChangedEvent>>();
}

/// Register all theme-related systems. Order matters for animation.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  use crate::ecs::schedule::IntoScheduleConfigs;

  schedule.add_systems(
    (
      systems::theme_command_system,
      systems::theme_animation_system,
      systems::theme_animation_update_system,
    )
      .chain(),
  );

  schedule.add_systems((
    systems::theme_change_detection_system,
    systems::theme_overrcodelord_system,
    systems::theme_hot_reload_system,
  ));
}

/// Theme animation resource
///
/// Smoothly transitions between themes using interpolation.
#[derive(Resource, Clone)]
pub struct ThemeAnimation {
  pub current_colors: AnimatedThemeColors,
  pub target_colors: AnimatedThemeColors,
  pub elapsed: f32,
  pub duration: f32,
  pub is_complete: bool,
}

impl ThemeAnimation {
  /// Create new animation from current to target theme
  pub fn new(from: &Theme, to: &Theme, duration: f32) -> Self {
    Self {
      current_colors: AnimatedThemeColors::from_theme(from),
      target_colors: AnimatedThemeColors::from_theme(to),
      elapsed: 0.0,
      duration,
      is_complete: false,
    }
  }

  /// Update animation by delta time
  pub fn update(&mut self, delta: f32) {
    if self.is_complete {
      return;
    }

    self.elapsed += delta;

    if self.elapsed >= self.duration {
      self.current_colors = self.target_colors.clone();
      self.is_complete = true;
      return;
    }

    // Normalize time and apply smoothstep
    let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
    let eased_t = Interpolation::InOutSmooth.y(t);

    // Interpolate all colors
    self.current_colors =
      self.current_colors.lerp(&self.target_colors, eased_t);
  }
}
