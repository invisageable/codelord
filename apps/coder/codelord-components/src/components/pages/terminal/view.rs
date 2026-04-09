//! Terminal view - Pure ECS terminal rendering.
//!
//! This view follows codelord's design but with pure ECS architecture:
//! - Terminal entities with TerminalTab, TerminalGrid, TerminalCursor
//!   components
//! - TerminalBridges resource stores type-erased bridges
//! - Auto-creates default terminal on first render
//! - Syncs bridge content to ECS components each frame
//! - Animated cursor uses ContinuousAnimations for ECS-based repaint

use super::bridge::AlacrittyBridge;
use super::cursor_animation::AnimatedCursor;

use crate::components::navigation::tabbar;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::query::With;
use codelord_core::ecs::world::World;
use codelord_core::keyboard::components::Focusable;
use codelord_core::keyboard::resources::KeyboardFocus;
use codelord_core::panel::resources::BottomPanelResource;

use codelord_core::terminal::{
  CELL_FLAG_DIM, CELL_FLAG_REVERSE, CELL_FLAG_STRIKETHROUGH,
  CELL_FLAG_UNDERLINE, TerminalBridges, TerminalCursor, TerminalGrid,
  TerminalRegistry, TerminalTab, unpack_color,
};

use codelord_core::ui::component::Active;

use eframe::egui;

use std::sync::Arc;

/// Terminal view state stored in egui memory.
/// Contains animated cursor for each terminal.
#[derive(Clone, Default)]
struct TerminalViewState {
  animated_cursor: AnimatedCursor,
}

/// Main terminal page view (includes tabbar).
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Terminal tabbar
  egui::Panel::top("terminal_tabbar")
    .frame(egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill))
    .exact_size(24.0)
    .resizable(false)
    .show_separator_line(true)
    .show_inside(ui, |ui| tabbar::show::<TerminalTab>(ui, world));

  // Terminal content
  egui::CentralPanel::default()
    .frame(
      egui::Frame::NONE.fill(ui.ctx().global_style().visuals.extreme_bg_color),
    )
    .show_inside(ui, |ui| {
      show_content(ui, world);
    });
}

/// Terminal content only (without tabbar) - used by views/terminal.rs.
pub fn show_content(ui: &mut egui::Ui, world: &mut World) {
  // Check if any terminal tabs exist
  let terminals: Vec<Entity> = world
    .query_filtered::<Entity, With<TerminalTab>>()
    .iter(world)
    .collect();

  if terminals.is_empty() {
    create_default_terminal(world);
  }

  // Re-query after potential creation
  let terminals: Vec<Entity> = world
    .query_filtered::<Entity, With<TerminalTab>>()
    .iter(world)
    .collect();

  ensure_terminal_bridges(world, &terminals);
  sync_terminals_to_ecs(world, &terminals);
  show_terminal_content(ui, world);
}

/// Create default terminal immediately (like codelord's new_terminal).
fn create_default_terminal(world: &mut World) {
  use codelord_core::tabbar::Tab;
  use codelord_core::terminal::{
    TerminalIdCounter, TerminalRegistry, TerminalScroll,
    TerminalTabOrderCounter,
  };

  // Get next ID
  let terminal_id = world
    .get_resource_mut::<TerminalIdCounter>()
    .map(|mut c| c.bump())
    .unwrap_or(codelord_core::terminal::TerminalId(0));

  // Get next order
  let order = world
    .get_resource_mut::<TerminalTabOrderCounter>()
    .map(|mut c| c.allocate())
    .unwrap_or(0);

  // Spawn terminal entity with all components
  let entity = world
    .spawn((
      Tab::new(format!("Terminal {}", terminal_id.0 + 1), order),
      TerminalTab,
      TerminalGrid::default(),
      TerminalCursor::default(),
      TerminalScroll::default(),
      Focusable,
      Active,
    ))
    .id();

  // Register in registry
  if let Some(mut registry) = world.get_resource_mut::<TerminalRegistry>() {
    registry.register(entity, terminal_id);
  }
}

/// Ensure all terminal entities have bridges.
fn ensure_terminal_bridges(world: &mut World, terminals: &[Entity]) {
  for &entity in terminals {
    // Check if bridge exists
    let has_bridge = world
      .get_resource::<TerminalBridges>()
      .map(|b| b.contains(entity))
      .unwrap_or(false);

    if !has_bridge {
      // Get terminal ID for logging
      let terminal_id = world
        .get_resource::<TerminalRegistry>()
        .and_then(|r| r.get_id(entity));

      // Create new bridge
      match AlacrittyBridge::new(24, 80, None) {
        Ok(bridge) => {
          if let Some(mut bridges) = world.get_resource_mut::<TerminalBridges>()
          {
            bridges.insert(entity, bridge);
          }
        }
        Err(e) => {
          eprintln!(
            "Failed to create terminal bridge for {terminal_id:?}: {e}",
          );
        }
      }
    }
  }
}

/// Sync terminal bridges to ECS components.
fn sync_terminals_to_ecs(world: &mut World, terminals: &[Entity]) {
  for &entity in terminals {
    let bridge: Option<Arc<AlacrittyBridge>> = world
      .get_resource::<TerminalBridges>()
      .and_then(|b| b.get::<AlacrittyBridge>(entity));

    if let Some(bridge) = bridge {
      bridge.process_events();

      let content = bridge.sync();

      if let Some(mut grid) = world.get_mut::<TerminalGrid>(entity) {
        *grid = content.grid;
      }

      if let Some(mut cursor) = world.get_mut::<TerminalCursor>(entity) {
        cursor.row = content.cursor_row;
        cursor.col = content.cursor_col;
        cursor.visible = content.cursor_visible;
      }
    }
  }
}

/// Show active terminal content.
fn show_terminal_content(ui: &mut egui::Ui, world: &mut World) {
  let panel_visible = world
    .get_resource::<BottomPanelResource>()
    .map(|p| p.is_visible)
    .unwrap_or(false);

  let visible_state_id = ui.id().with("terminal_visible_state");

  let was_visible =
    ui.memory(|m| m.data.get_temp(visible_state_id).unwrap_or(false));

  let just_became_visible = panel_visible && !was_visible;

  ui.memory_mut(|m| m.data.insert_temp(visible_state_id, panel_visible));

  let active_terminal = world
    .query_filtered::<Entity, (With<TerminalTab>, With<Active>)>()
    .iter(world)
    .next();

  let Some(entity) = active_terminal else {
    // No active terminal - pick first one
    let first_terminal: Option<Entity> = world
      .query_filtered::<Entity, With<TerminalTab>>()
      .iter(world)
      .next();

    if let Some(entity) = first_terminal {
      world.entity_mut(entity).insert(Active);
    }

    ui.centered_and_justified(|ui| {
      ui.label("No active terminal");
    });

    return;
  };

  // When terminal panel just became visible, set focus immediately
  // so the pre-consume block below catches it on the same frame.
  if just_became_visible
    && let Some(mut focus) = world.get_resource_mut::<KeyboardFocus>()
  {
    focus.set(entity);
  }

  // Pre-consume terminal keys BEFORE any widgets render.
  // This prevents egui's Tab navigation from stealing focus.
  let has_focus_early = world
    .get_resource::<KeyboardFocus>()
    .map(|f| f.has_focus(entity))
    .unwrap_or(false);

  if has_focus_early {
    let bridge_early: Option<Arc<AlacrittyBridge>> = world
      .get_resource::<TerminalBridges>()
      .and_then(|b| b.get::<AlacrittyBridge>(entity));

    if let Some(bridge) = &bridge_early {
      ui.input_mut(|i| {
        i.events.retain(|event| match event {
          egui::Event::Text(text) => {
            bridge.send_input(text);
            false
          }
          egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
          } => {
            if let Some(seq) = key_to_escape_sequence(*key, *modifiers) {
              bridge.send_input(&seq);
              false
            } else {
              true
            }
          }
          egui::Event::Paste(text) => {
            bridge.send_input(text);
            false
          }
          _ => true,
        });
      });
    }
  }

  // Get terminal components
  let grid = world.get::<TerminalGrid>(entity).cloned();
  let cursor = world.get::<TerminalCursor>(entity).copied();

  let Some(grid) = grid else {
    return;
  };

  // Get bridge for input handling
  let bridge: Option<Arc<AlacrittyBridge>> = world
    .get_resource::<TerminalBridges>()
    .and_then(|b| b.get::<AlacrittyBridge>(entity));

  // Calculate dimensions
  let font_size = 12.0;
  let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);

  let (char_width, line_height) = ui.fonts_mut(|f| {
    let char_width = f.glyph_width(&font_id, 'm');
    let row_height = f.row_height(&font_id);

    (char_width, row_height)
  });

  let rect = ui.available_rect_before_wrap();
  let visuals = ui.ctx().global_style().visuals.clone();

  // Background - use theme's extreme_bg_color
  ui.painter()
    .rect_filled(rect, 0.0, visuals.extreme_bg_color);

  // Apply inner margin (top, bottom, left)
  let margin_top = 8.0;
  let margin_bottom = 8.0;
  let margin_left = 8.0;

  let content_rect = egui::Rect::from_min_max(
    egui::pos2(rect.min.x + margin_left, rect.min.y + margin_top),
    egui::pos2(rect.max.x, rect.max.y - margin_bottom),
  );

  // Handle resize based on content area
  let cols = (content_rect.width() / char_width) as u16;
  let rows = (content_rect.height() / line_height) as u16;

  (cols > 0 && rows > 0).then(|| {
    bridge.as_ref().map(|bridge| {
      // Check if size changed (store last size in egui memory)
      let last_size_id = ui.id().with("terminal_last_size");
      let last_size = ui.memory(|m| m.data.get_temp(last_size_id));

      (last_size != Some((cols, rows))).then(|| {
        bridge.resize(rows, cols);
        ui.memory_mut(|m| m.data.insert_temp(last_size_id, (cols, rows)));
      });
    })
  });

  // Render grid with theme colors
  render_grid(
    ui,
    &grid,
    content_rect,
    char_width,
    line_height,
    &font_id,
    &visuals,
  );

  // Load view state from egui memory (per-terminal state)
  let state_id = ui.id().with(entity).with("terminal_view_state");

  let mut state = ui.memory::<TerminalViewState>(|m| {
    m.data.get_temp(state_id).unwrap_or_default()
  });

  // Render animated cursor
  let cursor_animating = cursor
    .filter(|c| c.visible)
    .map(|cursor| {
      // Calculate cursor center position within content_rect
      let cursor_pixel_pos = (
        content_rect.min.x
          + (cursor.col as f32 * char_width)
          + (char_width / 2.0),
        content_rect.min.y
          + (cursor.row as f32 * line_height)
          + (line_height / 2.0),
      );

      // Get delta time from ECS resource
      let dt = world
        .get_resource::<DeltaTime>()
        .map(|t| t.delta())
        .unwrap_or(1.0 / 60.0);

      // Check if cursor moved
      (cursor_pixel_pos != state.animated_cursor.destination()).then(|| {
        state
          .animated_cursor
          .jump_to(cursor_pixel_pos, (char_width, line_height));
      });

      // Animate cursor
      let animating = state.animated_cursor.animate(
        dt,
        (char_width, line_height),
        false, // Don't skip animation
      );

      // Render animated cursor - use theme text color
      state
        .animated_cursor
        .render(&ui.painter_at(content_rect), visuals.text_color());

      animating
    })
    .unwrap_or(false);

  // Save view state back to egui memory
  ui.memory_mut(|m| m.data.insert_temp(state_id, state));

  // Handle input - use content_rect for interaction
  let response = ui.interact(
    content_rect,
    ui.id().with(entity).with("terminal_input"),
    egui::Sense::click(),
  );

  // Request focus when terminal is clicked.
  // (just_became_visible is handled earlier, before widget rendering.)
  if response.clicked()
    && let Some(mut focus) = world.get_resource_mut::<KeyboardFocus>()
  {
    focus.set(entity);
  }

  // Handle scroll when hovered (not just focused)
  if response.hovered() {
    handle_mouse_scroll(ui, &bridge);
  }

  // Mark terminal as active for continuous animation (ECS-based repaint)
  // This handles both cursor animation and terminal updates
  if let Some(mut continuous) = world.get_resource_mut::<ContinuousAnimations>()
  {
    continuous.set_terminal_active();
    // If cursor is still animating, ensure we keep requesting repaints.
    cursor_animating.then(|| continuous.set_terminal_active());
  }
}

/// Handle mouse scroll.
fn handle_mouse_scroll(
  ui: &mut egui::Ui,
  bridge: &Option<Arc<AlacrittyBridge>>,
) {
  let Some(bridge) = bridge else {
    return;
  };

  ui.input(|i| {
    for event in &i.events {
      if let egui::Event::MouseWheel { delta, unit, .. } = event {
        match unit {
          egui::MouseWheelUnit::Line => {
            let lines = delta.y.signum() * delta.y.abs().ceil();

            bridge.scroll(lines as i32);
          }
          egui::MouseWheelUnit::Point => {
            let lines = (delta.y / 12.0).round();

            if lines != 0.0 {
              bridge.scroll(lines as i32);
            }
          }
          _ => {}
        }
      }
    }
  });
}

/// Default terminal foreground color from bridge (r=229, g=229, b=229, a=255).
/// Format: (a << 24) | (r << 16) | (g << 8) | b
const DEFAULT_FG_COLOR: u32 = 0xFFE5E5E5;

/// Render terminal grid.
fn render_grid(
  ui: &mut egui::Ui,
  grid: &TerminalGrid,
  rect: egui::Rect,
  char_width: f32,
  line_height: f32,
  font_id: &egui::FontId,
  visuals: &egui::Visuals,
) {
  let painter = ui.painter();
  let theme_text_color = visuals.text_color();

  for row in 0..grid.height {
    for col in 0..grid.width {
      if let Some(cell) = grid.get_cell(row, col) {
        if cell.character == ' ' && cell.bg_color == 0 {
          continue; // Skip empty transparent cells
        }

        let x = rect.min.x + col as f32 * char_width;
        let y = rect.min.y + row as f32 * line_height;

        // Stop if we're outside the visible rect
        if y > rect.max.y || x > rect.max.x {
          continue;
        }

        let cell_rect = egui::Rect::from_min_size(
          egui::pos2(x, y),
          egui::vec2(char_width, line_height),
        );

        // Background
        if cell.bg_color != 0 {
          let (r, g, b, a) = unpack_color(cell.bg_color);
          if a > 0 {
            painter.rect_filled(
              cell_rect,
              0.0,
              egui::Color32::from_rgba_unmultiplied(r, g, b, a),
            );
          }
        }

        // Character
        if cell.character != ' ' && cell.character != '\0' {
          // Use theme text color for default foreground, otherwise use cell
          // color
          let mut color = if cell.fg_color == DEFAULT_FG_COLOR {
            theme_text_color
          } else {
            let (r, g, b, a) = unpack_color(cell.fg_color);
            egui::Color32::from_rgba_unmultiplied(r, g, b, a)
          };

          // Handle reverse video
          if cell.flags & CELL_FLAG_REVERSE != 0 {
            let (br, bg, bb, _) = unpack_color(cell.bg_color);
            color = egui::Color32::from_rgb(br, bg, bb);
          }

          // Handle dim
          if cell.flags & CELL_FLAG_DIM != 0 {
            color = color.gamma_multiply(0.5);
          }

          // Center text in cell like codelord
          let text_pos = egui::pos2(x + char_width / 2.0, y);
          painter.text(
            text_pos,
            egui::Align2::CENTER_TOP,
            cell.character,
            font_id.clone(),
            color,
          );

          // Underline
          if cell.flags & CELL_FLAG_UNDERLINE != 0 {
            painter.line_segment(
              [
                egui::pos2(x, y + line_height - 2.0),
                egui::pos2(x + char_width, y + line_height - 2.0),
              ],
              egui::Stroke::new(1.0, color),
            );
          }

          // Strikethrough
          if cell.flags & CELL_FLAG_STRIKETHROUGH != 0 {
            painter.line_segment(
              [
                egui::pos2(x, y + line_height / 2.0),
                egui::pos2(x + char_width, y + line_height / 2.0),
              ],
              egui::Stroke::new(1.0, color),
            );
          }
        }
      }
    }
  }
}

/// Convert egui key to terminal escape sequence.
/// Full Ctrl+key support like codelord.
fn key_to_escape_sequence(
  key: egui::Key,
  modifiers: egui::Modifiers,
) -> Option<String> {
  // Ctrl+key combinations
  if modifiers.ctrl {
    match key {
      egui::Key::A => return Some("\x01".into()),
      egui::Key::B => return Some("\x02".into()),
      egui::Key::C => return Some("\x03".into()),
      egui::Key::D => return Some("\x04".into()),
      egui::Key::E => return Some("\x05".into()),
      egui::Key::F => return Some("\x06".into()),
      egui::Key::G => return Some("\x07".into()),
      egui::Key::H => return Some("\x08".into()),
      egui::Key::I => return Some("\t".into()),
      egui::Key::J => return Some("\n".into()),
      egui::Key::K => return Some("\x0b".into()),
      egui::Key::L => return Some("\x0c".into()),
      egui::Key::M => return Some("\r".into()),
      egui::Key::N => return Some("\x0e".into()),
      egui::Key::O => return Some("\x0f".into()),
      egui::Key::P => return Some("\x10".into()),
      egui::Key::Q => return Some("\x11".into()),
      egui::Key::R => return Some("\x12".into()),
      egui::Key::S => return Some("\x13".into()),
      egui::Key::T => return Some("\x14".into()),
      egui::Key::U => return Some("\x15".into()),
      egui::Key::V => return Some("\x16".into()),
      egui::Key::W => return Some("\x17".into()),
      egui::Key::X => return Some("\x18".into()),
      egui::Key::Y => return Some("\x19".into()),
      egui::Key::Z => return Some("\x1a".into()),
      _ => {}
    }
  }

  // Standard keys and escape sequences
  match key {
    egui::Key::ArrowUp => Some("\x1b[A".into()),
    egui::Key::ArrowDown => Some("\x1b[B".into()),
    egui::Key::ArrowRight => Some("\x1b[C".into()),
    egui::Key::ArrowLeft => Some("\x1b[D".into()),
    egui::Key::Home => Some("\x1b[H".into()),
    egui::Key::End => Some("\x1b[F".into()),
    egui::Key::PageUp => Some("\x1b[5~".into()),
    egui::Key::PageDown => Some("\x1b[6~".into()),
    egui::Key::Insert => Some("\x1b[2~".into()),
    egui::Key::Delete => Some("\x1b[3~".into()),
    egui::Key::Enter => Some("\r".into()),
    egui::Key::Tab => Some("\t".into()),
    egui::Key::Backspace => Some("\x7f".into()),
    egui::Key::Escape => Some("\x1b".into()),
    egui::Key::F1 => Some("\x1bOP".into()),
    egui::Key::F2 => Some("\x1bOQ".into()),
    egui::Key::F3 => Some("\x1bOR".into()),
    egui::Key::F4 => Some("\x1bOS".into()),
    egui::Key::F5 => Some("\x1b[15~".into()),
    egui::Key::F6 => Some("\x1b[17~".into()),
    egui::Key::F7 => Some("\x1b[18~".into()),
    egui::Key::F8 => Some("\x1b[19~".into()),
    egui::Key::F9 => Some("\x1b[20~".into()),
    egui::Key::F10 => Some("\x1b[21~".into()),
    egui::Key::F11 => Some("\x1b[23~".into()),
    egui::Key::F12 => Some("\x1b[24~".into()),
    _ => None,
  }
}
