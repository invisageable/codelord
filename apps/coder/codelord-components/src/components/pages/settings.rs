//! Settings page.

use crate::assets::font;
use crate::assets::icon::icon_to_image;
use crate::assets::theme::get_theme;
use crate::components::atoms::stripe_button;

use codelord_core::icon::components::{Arrow, Icon};

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::world::World;
use codelord_core::settings::resources::{
  SettingItem, SettingsNavMode, SettingsResource,
};
use codelord_core::theme::resources::{ThemeAction, ThemeCommand};

use codelord_i18n::set_locale;

use eframe::egui;

const CATEGORY_HEIGHT: f32 = 28.0;
const LEFT_COLUMN_RATIO: f32 = 0.5;
const PADDING: f32 = 40.0;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let rect = ui.available_rect_before_wrap();
  let theme = get_theme(world);

  let primary_color = egui::Color32::from_rgba_unmultiplied(
    theme.primary[0],
    theme.primary[1],
    theme.primary[2],
    theme.primary[3],
  );

  let text_color = ui.style().visuals.text_color();
  let weak_text_color = ui.style().visuals.weak_text_color();

  // Get delta time
  let dt = world
    .get_resource::<DeltaTime>()
    .map(|t| t.delta())
    .unwrap_or(1.0 / 60.0);

  // Handle keyboard navigation
  handle_keyboard(ui, world);

  // Update focus bar animation
  let is_animating = world
    .get_resource_mut::<SettingsResource>()
    .map(|mut s| s.update_focus_bar(dt))
    .unwrap_or(false);

  if is_animating
    && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
  {
    anim.increment()
  }

  // Update animations
  let (category_animating, item_animating) = world
    .get_resource_mut::<SettingsResource>()
    .map(|mut settings| {
      let cat_anim = settings
        .category_name_animation
        .as_mut()
        .map(|a| a.update(dt))
        .unwrap_or(false);

      let item_anim = settings
        .item_description_animation
        .as_mut()
        .map(|a| a.update(dt))
        .unwrap_or(false);

      (cat_anim, item_anim)
    })
    .unwrap_or((false, false));

  if (category_animating || item_animating)
    && let Some(mut anim) = world.get_resource_mut::<ActiveAnimations>()
  {
    anim.increment();
  }

  // Get state for rendering
  let (
    nav_mode,
    _focused_category,
    _focused_item,
    focus_bar_y,
    animated_category_name,
    animated_item_description,
  ) = {
    let Some(settings) = world.get_resource::<SettingsResource>() else {
      return;
    };

    let animated_name = settings
      .category_name_animation
      .as_ref()
      .map(|a| a.visible_text());

    let animated_desc = settings
      .item_description_animation
      .as_ref()
      .map(|a| a.visible_text());

    (
      settings.nav_mode,
      settings.focused_category,
      settings.focused_item,
      settings.focus_bar_y,
      animated_name,
      animated_desc,
    )
  };

  // Header
  render_header(
    ui,
    rect,
    text_color,
    primary_color,
    nav_mode,
    animated_category_name.as_deref(),
  );

  // Main content area
  let content_rect = egui::Rect::from_min_max(
    egui::pos2(rect.left(), rect.top() + 50.0),
    rect.max,
  );

  let left_width = content_rect.width() * LEFT_COLUMN_RATIO;

  // Focus bar (focus_bar_y is absolute screen Y of row center)
  let focus_bar_rect = egui::Rect::from_min_size(
    egui::pos2(rect.left(), focus_bar_y - CATEGORY_HEIGHT / 2.0),
    egui::vec2(4.0, CATEGORY_HEIGHT),
  );
  ui.painter().rect_filled(
    focus_bar_rect,
    egui::CornerRadius::ZERO,
    primary_color,
  );

  // Bottom panel with buttons
  let footer_height = 48.0;
  let footer_rect = egui::Rect::from_min_size(
    egui::pos2(rect.left(), rect.bottom() - footer_height),
    egui::vec2(rect.width(), footer_height),
  );

  ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.add_space(PADDING);

      // CANCEL button (secondary/black)
      stripe_button::show_styled(
        ui,
        world,
        "CANCEL",
        egui::vec2(100.0, 30.0),
        stripe_button::ButtonStyle::Secondary,
      );

      ui.add_space(8.0);

      // SAVE button (primary/green)
      stripe_button::show_styled(
        ui,
        world,
        "SAVE",
        egui::vec2(100.0, 30.0),
        stripe_button::ButtonStyle::Primary,
      );

      ui.add_space(8.0);

      // BACK button (secondary/black)
      if stripe_button::show_styled(
        ui,
        world,
        "BACK",
        egui::vec2(100.0, 30.0),
        stripe_button::ButtonStyle::Secondary,
      )
      .clicked()
        && let Some(mut settings) = world.get_resource_mut::<SettingsResource>()
      {
        settings.back_to_categories();
      }
    });
  });

  // Left column (adjusted for footer)
  let left_rect = egui::Rect::from_min_size(
    content_rect.min,
    egui::vec2(left_width, content_rect.height() - footer_height),
  );

  ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
    render_left_column(ui, world, text_color, weak_text_color, primary_color);
  });

  // Right column (only in Items mode, adjusted for footer)
  if nav_mode == SettingsNavMode::Items {
    let right_rect = egui::Rect::from_min_size(
      egui::pos2(content_rect.left() + left_width, content_rect.top()),
      egui::vec2(
        content_rect.width() - left_width,
        content_rect.height() - footer_height,
      ),
    );

    ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
      render_right_column(
        ui,
        world,
        text_color,
        weak_text_color,
        animated_item_description.as_deref(),
      );
    });
  }
}

fn handle_keyboard(ui: &mut egui::Ui, world: &mut World) {
  ui.ctx().input(|input| {
    let mut action = None;

    if input.key_pressed(egui::Key::ArrowUp) {
      action = Some(KeyAction::Up);
    } else if input.key_pressed(egui::Key::ArrowDown) {
      action = Some(KeyAction::Down);
    } else if input.key_pressed(egui::Key::ArrowLeft) {
      action = Some(KeyAction::Left);
    } else if input.key_pressed(egui::Key::ArrowRight) {
      action = Some(KeyAction::Right);
    } else if input.key_pressed(egui::Key::Enter)
      || input.key_pressed(egui::Key::Space)
    {
      action = Some(KeyAction::Enter);
    } else if input.key_pressed(egui::Key::Escape) {
      action = Some(KeyAction::Escape);
    }

    if let Some(action) = action
      && let Some(mut settings) = world.get_resource_mut::<SettingsResource>()
    {
      match action {
        KeyAction::Up => {
          settings.navigate_up();
          settings.update_focus_bar_target();
        }
        KeyAction::Down => {
          settings.navigate_down();
          settings.update_focus_bar_target();
        }
        KeyAction::Left => {
          let was_theme = is_theme_selector(&settings);
          let was_language = is_language_selector(&settings);
          settings.selector_left();
          if was_language {
            update_locale(&settings);
          }
          if was_theme {
            world.write_message(ThemeCommand {
              action: ThemeAction::Toggle,
            });
          }
        }
        KeyAction::Right => {
          let was_theme = is_theme_selector(&settings);
          let was_language = is_language_selector(&settings);
          settings.selector_right();
          if was_language {
            update_locale(&settings);
          }
          if was_theme {
            world.write_message(ThemeCommand {
              action: ThemeAction::Toggle,
            });
          }
        }
        KeyAction::Enter => {
          if settings.nav_mode == SettingsNavMode::Categories {
            settings.enter_category();
            // Don't update target here - y_positions will be stale
            // Render code updates target after populating new positions
          } else {
            let was_theme = is_theme_selector(&settings);
            let is_clear_session = settings.is_clear_session_action();
            let is_microphone = settings.is_microphone_permission_action();
            let is_action = settings.activate_item();

            if was_theme {
              world.write_message(ThemeCommand {
                action: ThemeAction::Toggle,
              });
            }

            if is_action && is_clear_session {
              world.spawn(codelord_core::events::ClearSessionRequest);
            }

            if is_action && is_microphone {
              #[cfg(target_os = "macos")]
              {
                let _ = std::process::Command::new("open")
                  .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                  .spawn();
              }
            }
          }
        }
        KeyAction::Escape => {
          settings.back_to_categories();
          // Don't update target here - y_positions will be stale
          // Render code updates target after populating new positions
        }
      }
    }
  });
}

enum KeyAction {
  Up,
  Down,
  Left,
  Right,
  Enter,
  Escape,
}

/// Check if the current focused item is the Theme selector.
fn is_theme_selector(settings: &SettingsResource) -> bool {
  // Theme is in APPEARANCE category (index 1), first item (index 0)
  settings.nav_mode == SettingsNavMode::Items
    && settings.selected_category == 1
    && settings.focused_item == 0
}

fn is_language_selector(settings: &SettingsResource) -> bool {
  // Language is in APP category (index 0), first item (index 0)
  settings.nav_mode == SettingsNavMode::Items
    && settings.selected_category == 0
    && settings.focused_item == 0
}

/// Get locale code from language selector index.
fn get_locale_code(index: usize) -> &'static str {
  ["en", "fr", "zh-CN", "ja"]
    .get(index)
    .copied()
    .unwrap_or("en")
}

/// Update locale based on current language selector value.
fn update_locale(settings: &SettingsResource) {
  settings
    .focused_item_data()
    .and_then(|item| match item {
      SettingItem::Selector { selected, .. } => Some(*selected),
      _ => None,
    })
    .map(get_locale_code)
    .inspect(|locale| set_locale(locale));
}

fn render_header(
  ui: &mut egui::Ui,
  rect: egui::Rect,
  text_color: egui::Color32,
  primary_color: egui::Color32,
  nav_mode: SettingsNavMode,
  selected_category_name: Option<&str>,
) {
  let header_rect =
    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 40.0));

  ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
    let center_y = header_rect.center().y;
    let mut current_x = header_rect.left() + PADDING;

    let font_id =
      egui::FontId::new(28.0, egui::FontFamily::Name(font::SUISSE_INTL.into()));

    // "SETTiNGS"
    ui.painter().text(
      egui::pos2(current_x, center_y),
      egui::Align2::LEFT_CENTER,
      "SETTiNGS",
      font_id.clone(),
      text_color,
    );

    let settings_width = ui.fonts_mut(|f| {
      f.layout_no_wrap("SETTiNGS".to_string(), font_id.clone(), text_color)
        .rect
        .width()
    });

    current_x += settings_width;

    // Category name if in Items mode
    if nav_mode == SettingsNavMode::Items
      && let Some(name) = selected_category_name
    {
      current_x += 16.0;

      ui.painter().text(
        egui::pos2(current_x, center_y),
        egui::Align2::LEFT_CENTER,
        name,
        font_id.clone(),
        primary_color,
      );

      let name_width = ui.fonts_mut(|f| {
        f.layout_no_wrap(name.to_string(), font_id.clone(), primary_color)
          .rect
          .width()
      });

      current_x += name_width;
    }

    // Separator line
    let line_start_x = current_x + 16.0;
    ui.painter().line_segment(
      [
        egui::pos2(line_start_x, center_y),
        egui::pos2(header_rect.right() - PADDING, center_y),
      ],
      egui::Stroke::new(1.0, egui::Color32::from_gray(30)),
    );
  });
}

fn render_left_column(
  ui: &mut egui::Ui,
  world: &mut World,
  text_color: egui::Color32,
  weak_text_color: egui::Color32,
  primary_color: egui::Color32,
) {
  let (nav_mode, categories, focused_category, focused_item, selected_category) = {
    let Some(settings) = world.get_resource::<SettingsResource>() else {
      return;
    };
    (
      settings.nav_mode,
      settings.categories.clone(),
      settings.focused_category,
      settings.focused_item,
      settings.selected_category,
    )
  };

  egui::ScrollArea::vertical()
    .id_salt("settings_scroll_left")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      ui.add_space(50.0);

      let mut y_positions = Vec::new();

      match nav_mode {
        SettingsNavMode::Categories => {
          for (cat_idx, category) in categories.iter().enumerate() {
            let is_focused = cat_idx == focused_category;
            let color = if is_focused {
              primary_color
            } else {
              text_color
            };

            let row_response = ui.horizontal(|ui| {
              ui.add_space(PADDING);

              let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width() - PADDING, CATEGORY_HEIGHT),
                egui::Sense::click(),
              );

              // Category name
              ui.painter().text(
                egui::pos2(rect.left(), rect.center().y),
                egui::Align2::LEFT_CENTER,
                category.name(),
                egui::FontId::new(
                  14.0,
                  egui::FontFamily::Name(font::SUISSE_INTL.into()),
                ),
                color,
              );

              // Separator line
              let text_width = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                  category.name(),
                  egui::FontId::new(
                    14.0,
                    egui::FontFamily::Name(font::SUISSE_INTL.into()),
                  ),
                  color,
                )
                .rect
                .width()
              });

              let line_start = rect.left() + text_width + 16.0;
              ui.painter().line_segment(
                [
                  egui::pos2(line_start, rect.center().y),
                  egui::pos2(rect.right(), rect.center().y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_gray(30)),
              );

              if response.clicked()
                && let Some(mut s) =
                  world.get_resource_mut::<SettingsResource>()
              {
                s.focused_category = cat_idx;
                s.enter_category();
                s.update_focus_bar_target();
              }

              if response.hovered() {
                ui.output_mut(|o| {
                  o.cursor_icon = egui::CursorIcon::PointingHand
                });
              }
            });

            // Store absolute screen Y of row center
            y_positions.push(row_response.response.rect.center().y);

            ui.add_space(8.0);
          }
        }
        SettingsNavMode::Items => {
          if let Some(category) = categories.get(selected_category) {
            for (item_idx, item) in category.items.iter().enumerate() {
              let is_focused = item_idx == focused_item;
              let label_color = if is_focused {
                text_color
              } else {
                weak_text_color
              };

              let row_response = ui.horizontal(|ui| {
                ui.add_space(PADDING);

                ui.with_layout(
                  egui::Layout::left_to_right(egui::Align::Center),
                  |ui| {
                    // Label
                    ui.label(
                      egui::RichText::new(item.label())
                        .size(12.0)
                        .family(egui::FontFamily::Name(
                          font::SUISSE_INTL.into(),
                        ))
                        .color(label_color),
                    );

                    ui.with_layout(
                      egui::Layout::right_to_left(egui::Align::Center),
                      |ui| {
                        ui.add_space(PADDING);

                        match item {
                          SettingItem::Toggle { value, .. } => {
                            render_toggle(
                              ui,
                              *value,
                              is_focused,
                              primary_color,
                            );
                          }
                          SettingItem::Selector {
                            options, selected, ..
                          } => {
                            render_selector(
                              ui,
                              options,
                              *selected,
                              primary_color,
                            );
                          }
                          SettingItem::Text { value, .. } => {
                            render_text_value(ui, value, primary_color);
                          }
                          SettingItem::Action { action_label, .. } => {
                            render_action_button(
                              ui,
                              action_label,
                              primary_color,
                            );
                          }
                        }
                      },
                    );
                  },
                );
              });

              // Store absolute screen Y of row center
              y_positions.push(row_response.response.rect.center().y);

              ui.add_space(4.0);
            }
          }
        }
      }

      // Store Y positions and update focus bar target
      if let Some(mut settings) = world.get_resource_mut::<SettingsResource>() {
        let needs_init = settings.focus_bar_y == 0.0 && !y_positions.is_empty();
        settings.item_y_positions = y_positions;

        // Always update target to match current positions
        settings.update_focus_bar_target();

        if needs_init {
          // Snap immediately on first render
          settings.focus_bar_y = settings.focus_bar_target_y;
        }
      }
    });
}

fn render_toggle(
  ui: &mut egui::Ui,
  value: bool,
  _is_focused: bool,
  primary_color: egui::Color32,
) {
  let switch_size = egui::vec2(40.0, 18.0);
  let (rect, _response) =
    ui.allocate_exact_size(switch_size, egui::Sense::hover());

  let bg_color = if value {
    primary_color
  } else {
    egui::Color32::from_gray(60)
  };

  ui.painter()
    .rect_filled(rect, egui::CornerRadius::same(9), bg_color);

  let knob_x = if value {
    rect.right() - 9.0
  } else {
    rect.left() + 9.0
  };

  ui.painter().circle_filled(
    egui::pos2(knob_x, rect.center().y),
    7.0,
    egui::Color32::from_gray(240),
  );
}

fn render_selector(
  ui: &mut egui::Ui,
  options: &[&str],
  selected: usize,
  primary_color: egui::Color32,
) {
  ui.horizontal(|ui| {
    if let Some(value) = options.get(selected) {
      let icon_size = egui::vec2(12.0, 12.0);

      // Right arrow icon
      let at_end = selected >= options.len().saturating_sub(1);
      let right_tint = if at_end {
        egui::Color32::from_gray(60)
      } else {
        egui::Color32::from_gray(120)
      };
      ui.add(
        icon_to_image(&Icon::Arrow(Arrow::AngleRightLine))
          .fit_to_exact_size(icon_size)
          .tint(right_tint),
      );

      ui.add_space(2.0);

      // Left arrow icon
      let at_start = selected == 0;
      let left_tint = if at_start {
        egui::Color32::from_gray(60)
      } else {
        egui::Color32::from_gray(120)
      };
      ui.add(
        icon_to_image(&Icon::Arrow(Arrow::AngleLeftLine))
          .fit_to_exact_size(icon_size)
          .tint(left_tint),
      );

      ui.add_space(6.0);

      // Selected value
      ui.label(
        egui::RichText::new(*value)
          .size(12.0)
          .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
          .color(primary_color),
      );
    }
  });
}

fn render_text_value(
  ui: &mut egui::Ui,
  value: &str,
  primary_color: egui::Color32,
) {
  ui.label(
    egui::RichText::new(value)
      .size(12.0)
      .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
      .color(primary_color),
  );
}

fn render_action_button(
  ui: &mut egui::Ui,
  action_label: &str,
  primary_color: egui::Color32,
) {
  // Small button-like appearance
  let (rect, _response) =
    ui.allocate_exact_size(egui::vec2(60.0, 20.0), egui::Sense::hover());

  // Border
  ui.painter().rect_stroke(
    rect,
    egui::CornerRadius::same(4),
    egui::Stroke::new(1.0, primary_color),
    egui::StrokeKind::Inside,
  );

  // Label
  ui.painter().text(
    rect.center(),
    egui::Align2::CENTER_CENTER,
    action_label,
    egui::FontId::new(10.0, egui::FontFamily::Name(font::SUISSE_INTL.into())),
    primary_color,
  );
}

fn render_right_column(
  ui: &mut egui::Ui,
  world: &World,
  text_color: egui::Color32,
  weak_text_color: egui::Color32,
  animated_description: Option<&str>,
) {
  let Some(settings) = world.get_resource::<SettingsResource>() else {
    return;
  };

  let Some(item) = settings.focused_item_data() else {
    return;
  };

  // Use animated description if available, otherwise fall back to static
  let description = animated_description.unwrap_or_else(|| item.description());

  ui.horizontal(|ui| {
    ui.add_space(PADDING);

    ui.vertical(|ui| {
      ui.add_space(30.0);

      // Item label
      ui.label(
        egui::RichText::new(item.label())
          .size(24.0)
          .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
          .color(text_color),
      );

      ui.add_space(16.0);

      // Item description (animated)
      ui.label(
        egui::RichText::new(description)
          .size(14.0)
          .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
          .color(weak_text_color),
      );
    });
  });
}
