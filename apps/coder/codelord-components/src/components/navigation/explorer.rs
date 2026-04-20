use codelord_core::ecs::world::World;
use codelord_core::navigation::resources::ExplorerState;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  // Check if we have a project loaded
  let has_project = world
    .get_resource::<ExplorerState>()
    .map(|s| !s.roots.is_empty())
    .unwrap_or(false);

  if has_project {
    egui::Panel::top("explorer_header")
      .frame(
        egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill),
      )
      .exact_size(24.0)
      .resizable(false)
      .show_separator_line(true)
      .show_inside(ui, |ui| header::show(ui, world));

    egui::CentralPanel::default()
      .frame(
        egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill),
      )
      .show_inside(ui, |ui| file_tree::show(ui, world));
  } else {
    egui::CentralPanel::default()
      .frame(
        egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill),
      )
      .show_inside(ui, |ui| open_project::show(ui, world));
  }
}

pub mod header {
  use crate::assets::theme;
  use crate::components::atoms::icon_button;

  use codelord_core::animation::components::DeltaTime;
  use codelord_core::animation::resources::ActiveAnimations;
  use codelord_core::ecs::world::World;
  use codelord_core::events::{
    CollapseAllFoldersRequest, RefreshExplorerRequest, ToggleHiddenFilesRequest,
  };
  use codelord_core::icon::components::{Byakugan, Icon};
  use codelord_core::navigation::components::FileEntry;
  use codelord_core::navigation::resources::{
    ExplorerItemsCounter, ExplorerState,
  };

  use eframe::egui;

  pub fn show(ui: &mut egui::Ui, world: &mut World) {
    // Get show_hidden state for byakugan icon
    let show_hidden = world
      .get_resource::<ExplorerState>()
      .map(|s| s.show_hidden)
      .unwrap_or(false);

    let theme = theme::get_theme(world);

    // Count file entries
    let entry_count = world.query::<&FileEntry>().iter(world).count();
    let delta = world
      .get_resource::<DeltaTime>()
      .map(|dt| dt.delta())
      .unwrap_or(0.016);

    // Update counter animation (follows LineColumnAnimation pattern)
    let (animated_count, started, completed) = if let Some(mut items_counter) =
      world.get_resource_mut::<ExplorerItemsCounter>()
    {
      // set_target returns true if animation started (wasn't already active)
      let started = items_counter.set_target(entry_count);
      // update returns true if animation completed
      let completed = items_counter.update(delta);
      (items_counter.count, started, completed)
    } else {
      (entry_count, false, false)
    };

    // Track animation via ActiveAnimations (after releasing items_counter
    // borrow)
    if (started || completed)
      && let Some(mut active_anims) =
        world.get_resource_mut::<ActiveAnimations>()
    {
      if started {
        active_anims.increment();
      }
      if completed {
        active_anims.decrement();
      }
    }

    ui.horizontal_centered(|ui| {
      ui.set_height(ui.available_height());

      ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add_space(8.0);

        ui.label(
          egui::RichText::new(format!("{animated_count} items"))
            .color(egui::Color32::GRAY)
            .size(10.0),
        );
      });

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);

        let text_color = egui::Color32::from_rgba_unmultiplied(
          theme.text[0],
          theme.text[1],
          theme.text[2],
          theme.text[3],
        );
        let primary_color = egui::Color32::from_rgba_unmultiplied(
          theme.primary[0],
          theme.primary[1],
          theme.primary[2],
          theme.primary[3],
        );

        // Toggle hidden files (byakugan)
        let byakugan_icon = if show_hidden {
          Icon::Byakugan(Byakugan::On)
        } else {
          Icon::Byakugan(Byakugan::Off)
        };
        let byakugan_tint = if show_hidden {
          primary_color
        } else {
          text_color
        };
        if icon_button::show(ui, &byakugan_icon, byakugan_tint) {
          world.spawn(ToggleHiddenFilesRequest);
        }

        // Collapse all folders
        if icon_button::show(ui, &Icon::Collapse, text_color) {
          world.spawn(CollapseAllFoldersRequest);
        }

        // Refresh explorer
        if icon_button::show(ui, &Icon::Refresh, text_color) {
          world.spawn(RefreshExplorerRequest);
        }
      });
    });
  }
}

pub mod file_tree {
  use crate::assets::icon;

  use codelord_core::animation::resources::ActiveAnimations;
  use codelord_core::button::components::{Button, ButtonContent};
  use codelord_core::ecs::entity::Entity;
  use codelord_core::ecs::world::World;
  use codelord_core::events::{
    CollapseFolderRequest, ExpandFolderRequest, OpenFileRequest,
  };
  use codelord_core::events::{
    CreateFileRequest, CreateFolderRequest, RenameRequest,
  };
  use codelord_core::icon::components::{Icon, Structure};
  use codelord_core::navigation::components::{Expanded, FileEntry, Selected};
  use codelord_core::navigation::resources::{
    ActiveWorkspaceRoot, ExplorerContextTarget, ExplorerEditingMode,
    ExplorerEditingState, ExplorerState, IndentationLinesState,
  };
  use codelord_core::popup::resources::{
    PopupAction, PopupCommand, PopupResource,
  };

  use eazy::{Curve, Easing};
  use eframe::egui;

  use rustc_hash::FxHashMap as HashMap;

  use std::path::PathBuf;

  /// Entry height matching codelord.
  const ENTRY_HEIGHT: f32 = 22.0;
  /// Base left margin before icon (matches codelord's ICON_WIDTH = 6.0).
  const ICON_OFFSET: f32 = 6.0;
  /// Indentation per depth level.
  const INDENT_WIDTH: f32 = 16.0;
  /// Centering offset for tree lines.
  const CENTERING_OFFSET: f32 = 10.0;
  /// Horizontal branch length.
  const BRANCH_LENGTH: f32 = 8.0;
  /// Fade animation duration in seconds.
  const FADE_DURATION: f64 = 0.3;

  struct TreeEntry {
    entity: Entity,
    entry: FileEntry,
    is_expanded: bool,
    is_selected: bool,
  }

  /// Info for rendering indentation lines.
  struct EntryRenderInfo {
    depth: u32,
    start_y: f32,
    end_y: f32,
  }

  pub fn show(ui: &mut egui::Ui, world: &mut World) {
    // Collect all entries
    let entries: Vec<TreeEntry> = world
      .query::<(Entity, &FileEntry)>()
      .iter(world)
      .map(|(entity, entry)| TreeEntry {
        entity,
        entry: entry.clone(),
        is_expanded: world.get::<Expanded>(entity).is_some(),
        is_selected: world.get::<Selected>(entity).is_some(),
      })
      .collect();

    // Group entries by parent
    let mut children_map: HashMap<Option<PathBuf>, Vec<TreeEntry>> =
      HashMap::default();
    for entry in entries {
      children_map
        .entry(entry.entry.parent.clone())
        .or_default()
        .push(entry);
    }

    // Sort each group: directories first, then alphabetically
    for group in children_map.values_mut() {
      group.sort_by(|a, b| match (a.entry.is_dir, b.entry.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => {
          let a_name = a.entry.name().to_lowercase();
          let b_name = b.entry.name().to_lowercase();
          a_name.cmp(&b_name)
        }
      });
    }

    // Collect clicks and entry positions for line rendering
    let mut clicks: Vec<(Entity, FileEntry, bool)> = Vec::new();
    let mut entry_infos: Vec<EntryRenderInfo> = Vec::new();

    let scroll_response = egui::ScrollArea::vertical()
      .id_salt("explorer_scrollbar")
      .auto_shrink([false; 2])
      .show(ui, |ui| {
        // Remove default vertical spacing between items.
        ui.spacing_mut().item_spacing.y = 0.0;

        render_level(
          ui,
          world,
          &children_map,
          None,
          &mut clicks,
          &mut entry_infos,
        )
      });

    // Track hover state for indentation lines animation
    let content_rect = scroll_response.inner_rect;
    let is_hovered = ui.rect_contains_pointer(content_rect);
    let current_time = ui.input(|i| i.time);

    // Update hover animation state
    let (lines_animating, animation_started) = {
      if let Some(mut lines_state) =
        world.get_resource_mut::<IndentationLinesState>()
      {
        let started = lines_state.set_hovered(is_hovered, current_time);
        let animating = lines_state.is_animating(current_time, FADE_DURATION);
        (animating, started)
      } else {
        (false, false)
      }
    };

    // Increment active animations if animation just started
    if animation_started
      && let Some(mut active_anims) =
        world.get_resource_mut::<ActiveAnimations>()
    {
      active_anims.increment();
    }

    // Calculate line opacity based on animation state
    let line_opacity = calculate_line_opacity(world, current_time);

    // Render indentation lines on background layer
    if line_opacity > 0.0 && !entry_infos.is_empty() {
      let left_x = scroll_response.inner_rect.left();
      let visuals = ui.style().visuals.clone();

      // Use theme color for lines (separator color with opacity).
      let line_color = visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        .linear_multiply(line_opacity);

      // Create background painter for lines
      let ctx = ui.ctx();
      let layer_id = ui.layer_id();
      let clip_rect = scroll_response.inner_rect;

      let mut bg_painter = ctx.layer_painter(egui::LayerId::new(
        layer_id.order,
        egui::Id::new("explorer_bg_lines"),
      ));
      bg_painter.set_clip_rect(clip_rect);

      render_indentation_lines_behind(
        &bg_painter,
        left_x,
        &entry_infos,
        line_color,
      );
    }

    // Decrement active animations when fade completes (only once)
    if !lines_animating {
      let should_decrement = world
        .get_resource::<IndentationLinesState>()
        .map(|s| s.needs_decrement)
        .unwrap_or(false);

      if should_decrement {
        if let Some(mut active_anims) =
          world.get_resource_mut::<ActiveAnimations>()
        {
          active_anims.decrement();
        }
        if let Some(mut lines_state) =
          world.get_resource_mut::<IndentationLinesState>()
        {
          lines_state.needs_decrement = false;
        }
      }
    }

    // Empty space context menu (right-click below entries to create at root)
    let content_bottom_y =
      scroll_response.inner_rect.min.y + scroll_response.content_size.y;
    let empty_space_rect = egui::Rect::from_min_max(
      egui::pos2(scroll_response.inner_rect.min.x, content_bottom_y),
      scroll_response.inner_rect.max,
    );

    if empty_space_rect.height() > 0.0 {
      let empty_space_response = ui.interact(
        empty_space_rect,
        egui::Id::new("explorer_empty_space"),
        egui::Sense::click(),
      );

      // Right-click on empty space shows context menu for root
      if empty_space_response.secondary_clicked() {
        // Get primary root path from ExplorerState
        let root_path = world
          .get_resource::<ExplorerState>()
          .and_then(|s| s.root_path().cloned());

        if let Some(path) = root_path {
          // Update context target to root
          if let Some(mut target) =
            world.get_resource_mut::<ExplorerContextTarget>()
          {
            target.entity = None;
            target.path = Some(path);
            target.is_dir = true;
          }

          // Show context menu popup
          if let Some(popup_entity) = world
            .get_resource::<PopupResource>()
            .and_then(|r| r.explorer_context_popup)
          {
            let cursor_pos = ui.input(|i| {
              i.pointer.hover_pos().unwrap_or(empty_space_rect.center())
            });

            world.write_message(PopupCommand {
              action: PopupAction::Show {
                entity: popup_entity,
                anchor_rect: [cursor_pos.x, cursor_pos.y, 0.0, 0.0],
              },
            });
          }
        }
      }
    }

    // Process clicks - spawn events for systems to handle
    for (entity, entry, was_expanded) in clicks {
      if was_expanded {
        world.spawn(CollapseFolderRequest::new(entity, entry.path));
      } else {
        world.spawn(ExpandFolderRequest::new(entity, entry.path, entry.depth));
      }
    }
  }

  /// Calculate line opacity based on hover animation.
  fn calculate_line_opacity(world: &World, current_time: f64) -> f32 {
    let Some(lines_state) = world.get_resource::<IndentationLinesState>()
    else {
      return 0.0;
    };

    let Some(start_time) = lines_state.hover_start_time else {
      return 0.0;
    };

    let elapsed = current_time - start_time;

    if elapsed >= FADE_DURATION {
      // Animation complete
      if lines_state.fading_in { 1.0 } else { 0.0 }
    } else {
      // Animation in progress
      let t = (elapsed / FADE_DURATION) as f32;
      if lines_state.fading_in {
        Easing::InOutCubic.y(t)
      } else {
        1.0 - Easing::InCubic.y(t)
      }
    }
  }

  /// Renders tree-style indentation lines.
  fn render_indentation_lines_behind(
    painter: &egui::Painter,
    left_x: f32,
    entry_infos: &[EntryRenderInfo],
    line_color: egui::Color32,
  ) {
    if entry_infos.is_empty() {
      return;
    }

    let line_stroke = egui::Stroke::new(1.0_f32, line_color);

    for (i, entry) in entry_infos.iter().enumerate() {
      if entry.depth == 0 {
        continue; // Root level entries don't need tree characters
      }

      let center_y = (entry.start_y + entry.end_y) / 2.0;

      // Check if this is the last sibling
      let is_last_sibling = {
        let mut found_sibling = false;
        for future_entry in entry_infos.iter().skip(i + 1) {
          if future_entry.depth < entry.depth {
            break;
          }
          if future_entry.depth == entry.depth {
            found_sibling = true;
            break;
          }
        }
        !found_sibling
      };

      // Draw tree characters for each depth level
      for depth_level in 0..entry.depth {
        let base_x = left_x
          + (depth_level as f32 * INDENT_WIDTH)
          + ICON_OFFSET
          + CENTERING_OFFSET;

        if depth_level == entry.depth - 1 {
          // Immediate parent level - draw the branch
          if is_last_sibling {
            // └─ shape
            painter.line_segment(
              [
                egui::pos2(base_x, entry.start_y),
                egui::pos2(base_x, center_y),
              ],
              line_stroke,
            );
          } else {
            // ├─ shape
            painter.line_segment(
              [
                egui::pos2(base_x, entry.start_y),
                egui::pos2(base_x, entry.end_y),
              ],
              line_stroke,
            );
          }

          // Horizontal branch
          painter.line_segment(
            [
              egui::pos2(base_x, center_y),
              egui::pos2(base_x + BRANCH_LENGTH, center_y),
            ],
            line_stroke,
          );
        } else {
          // Ancestor level - check if vertical line needed
          let needs_vertical = entry_infos
            .iter()
            .skip(i + 1)
            .any(|future| future.depth == depth_level + 1);

          if needs_vertical {
            // │ continuation line
            painter.line_segment(
              [
                egui::pos2(base_x, entry.start_y),
                egui::pos2(base_x, entry.end_y),
              ],
              line_stroke,
            );
          }
        }
      }
    }
  }

  /// Render tree recursively, collecting entry positions.
  fn render_level(
    ui: &mut egui::Ui,
    world: &mut World,
    children_map: &HashMap<Option<PathBuf>, Vec<TreeEntry>>,
    parent: Option<PathBuf>,
    clicks: &mut Vec<(Entity, FileEntry, bool)>,
    entry_infos: &mut Vec<EntryRenderInfo>,
  ) {
    let visuals = ui.style().visuals.clone();

    let Some(children) = children_map.get(&parent) else {
      return;
    };

    for tree_entry in children {
      let indent = tree_entry.entry.depth as f32 * INDENT_WIDTH;

      // Allocate full-width row
      let available_width = ui.available_width();
      let (row_id, row_rect) =
        ui.allocate_space(egui::vec2(available_width, ENTRY_HEIGHT));
      let row_response = ui.interact(row_rect, row_id, egui::Sense::click());

      // Collect entry info for line rendering
      entry_infos.push(EntryRenderInfo {
        depth: tree_entry.entry.depth,
        start_y: row_rect.top(),
        end_y: row_rect.bottom(),
      });

      let is_hovered = row_response.hovered();
      let is_selected = tree_entry.is_selected;

      // Button hover colors (mapped via weak_bg_fill in theme.rs).
      let hover_bg = visuals.widgets.hovered.weak_bg_fill;
      let hover_fg = visuals.widgets.active.weak_bg_fill;
      // Selection highlight - subtle light gray.
      let selected_bg =
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15);
      let normal_icon = visuals.widgets.inactive.fg_stroke.color;

      // Full-span highlighting using theme colors.
      if is_hovered {
        ui.painter()
          .rect_filled(row_rect, egui::CornerRadius::ZERO, hover_bg);
      } else if is_selected {
        ui.painter().rect_filled(
          row_rect,
          egui::CornerRadius::ZERO,
          selected_bg,
        );
      }

      // Colors: inverted on hover, gray for hidden files, normal otherwise.
      let icon_color = if is_hovered { hover_fg } else { normal_icon };

      // Hidden files are displayed in gray (matching codelord).
      let hidden_color = egui::Color32::from_rgb(120, 120, 120);

      let text_color = if is_hovered {
        hover_fg
      } else if tree_entry.entry.is_hidden {
        hidden_color
      } else {
        visuals.text_color()
      };

      // Position content within row
      let mut cursor_x = row_rect.left() + indent + ICON_OFFSET;

      // Draw icon
      let icon_rect = egui::Rect::from_min_size(
        egui::pos2(cursor_x, row_rect.top()),
        egui::vec2(20.0, ENTRY_HEIGHT),
      );

      if let Some(button) = world.get::<Button>(tree_entry.entity)
        && let ButtonContent::IconLabel(icn, _) = &button.content
      {
        ui.put(
          icon_rect,
          icon::icon_to_image(icn)
            .fit_to_exact_size(egui::Vec2::splat(12.0))
            .tint(icon_color),
        );
      }

      cursor_x += 24.0;

      // Check if we're renaming this entry
      let is_renaming = world
        .get_resource::<ExplorerEditingState>()
        .map(|state| {
          state.mode == Some(ExplorerEditingMode::Rename)
            && state.target_entity == Some(tree_entry.entity)
        })
        .unwrap_or(false);

      if is_renaming {
        // Render inline text edit for rename
        let text_rect = egui::Rect::from_min_size(
          egui::pos2(cursor_x, row_rect.top()),
          egui::vec2(
            row_rect.width() - cursor_x + row_rect.left(),
            ENTRY_HEIGHT,
          ),
        );

        let mut text = world
          .get_resource::<ExplorerEditingState>()
          .map(|s| s.text.clone())
          .unwrap_or_default();

        let response = ui.put(
          text_rect,
          egui::TextEdit::singleline(&mut text)
            .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
            .frame(egui::Frame::default())
            .margin(egui::Margin::symmetric(4, 2)),
        );

        // Request focus on first frame
        response.request_focus();

        // Update text in state
        if let Some(mut state) =
          world.get_resource_mut::<ExplorerEditingState>()
        {
          state.text = text.clone();
        }

        // Handle Enter to confirm, Escape to cancel
        if response.lost_focus() {
          let submitted = ui.input(|i| i.key_pressed(egui::Key::Enter));
          let cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));

          if submitted && !text.is_empty() {
            let parent_path = world
              .get_resource::<ExplorerEditingState>()
              .and_then(|state| state.parent_path.clone());

            if let Some(parent) = parent_path {
              let old_path = parent.join(tree_entry.entry.name());
              world.spawn(RenameRequest::new(
                tree_entry.entity,
                old_path,
                text.clone(),
              ));
            }
          }

          // Clear editing state
          if (submitted || cancelled)
            && let Some(mut state) =
              world.get_resource_mut::<ExplorerEditingState>()
          {
            state.mode = None;
          }
        }
      } else {
        // Draw text normally
        let name = tree_entry.entry.name();
        let text_pos = egui::pos2(cursor_x, row_rect.center().y);
        ui.painter().text(
          text_pos,
          egui::Align2::LEFT_CENTER,
          name,
          egui::FontId::new(12.0, egui::FontFamily::Proportional),
          text_color,
        );

        // Cursor icon on hover
        if is_hovered {
          ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
      }

      // Handle left click
      if row_response.clicked() {
        // Update active workspace based on clicked item (for both files and
        // folders)
        let clicked_path = tree_entry.entry.path.clone();
        let workspace_update =
          world.get_resource::<ExplorerState>().and_then(|explorer| {
            explorer.find_root_for_path(&clicked_path).map(|root| {
              let name =
                root.file_name().map(|n| n.to_string_lossy().to_string());
              (root.clone(), name)
            })
          });

        if let Some((root_path, name)) = workspace_update
          && let Some(mut active_ws) =
            world.get_resource_mut::<ActiveWorkspaceRoot>()
        {
          active_ws.path = Some(root_path);
          active_ws.name = name;
        }

        // Handle folder expand/collapse or file open
        if tree_entry.entry.is_dir {
          clicks.push((
            tree_entry.entity,
            tree_entry.entry.clone(),
            tree_entry.is_expanded,
          ));
        } else {
          world.spawn(OpenFileRequest::new(tree_entry.entry.path.clone()));
        }
      }

      // Handle right click - show context menu
      if row_response.secondary_clicked() {
        // Update context target
        if let Some(mut target) =
          world.get_resource_mut::<ExplorerContextTarget>()
        {
          target.entity = Some(tree_entry.entity);
          target.path = Some(tree_entry.entry.path.clone());
          target.is_dir = tree_entry.entry.is_dir;
        }

        // Show context menu popup at cursor position
        if let Some(popup_entity) = world
          .get_resource::<PopupResource>()
          .and_then(|r| r.explorer_context_popup)
        {
          let cursor_pos =
            ui.input(|i| i.pointer.hover_pos().unwrap_or(row_rect.center()));

          world.write_message(PopupCommand {
            action: PopupAction::Show {
              entity: popup_entity,
              anchor_rect: [cursor_pos.x, cursor_pos.y, 0.0, 0.0],
            },
          });
        }
      }

      // Render children if expanded
      if tree_entry.is_expanded && tree_entry.entry.is_dir {
        render_level(
          ui,
          world,
          children_map,
          Some(tree_entry.entry.path.clone()),
          clicks,
          entry_infos,
        );

        // Check if we need to render new file/folder input in this directory
        let should_render_new_input = world
          .get_resource::<ExplorerEditingState>()
          .map(|state| {
            (state.mode == Some(ExplorerEditingMode::NewFile)
              || state.mode == Some(ExplorerEditingMode::NewFolder))
              && state.parent_path.as_ref() == Some(&tree_entry.entry.path)
          })
          .unwrap_or(false);

        if should_render_new_input {
          render_new_input_row(ui, world, tree_entry.entry.depth + 1, &visuals);
        }
      }
    }
  }

  /// Render an inline text input row for new file/folder creation.
  fn render_new_input_row(
    ui: &mut egui::Ui,
    world: &mut World,
    depth: u32,
    visuals: &egui::Visuals,
  ) {
    let indent = depth as f32 * INDENT_WIDTH;
    let available_width = ui.available_width();
    let (_row_id, row_rect) =
      ui.allocate_space(egui::vec2(available_width, ENTRY_HEIGHT));

    // Background highlight
    ui.painter().rect_filled(
      row_rect,
      egui::CornerRadius::ZERO,
      visuals.widgets.hovered.weak_bg_fill,
    );

    let cursor_x = row_rect.left() + indent + ICON_OFFSET;

    // Get editing mode for icon
    let mode = world
      .get_resource::<ExplorerEditingState>()
      .and_then(|s| s.mode);

    // Draw icon (folder or file)
    let icon_rect = egui::Rect::from_min_size(
      egui::pos2(cursor_x, row_rect.top()),
      egui::vec2(20.0, ENTRY_HEIGHT),
    );

    let icon = mode
      .map(|m| match m {
        ExplorerEditingMode::NewFolder => {
          Icon::Structure(Structure::FolderClose)
        }
        _ => Icon::Structure(Structure::File),
      })
      .unwrap_or(Icon::Structure(Structure::File));

    ui.put(
      icon_rect,
      icon::icon_to_image(&icon)
        .fit_to_exact_size(egui::Vec2::splat(12.0))
        .tint(visuals.widgets.active.weak_bg_fill),
    );

    let text_x = cursor_x + 24.0;

    // Text input
    let text_rect = egui::Rect::from_min_size(
      egui::pos2(text_x, row_rect.top()),
      egui::vec2(row_rect.width() - text_x + row_rect.left(), ENTRY_HEIGHT),
    );

    let mut text = world
      .get_resource::<ExplorerEditingState>()
      .map(|s| s.text.clone())
      .unwrap_or_default();

    let response = ui.put(
      text_rect,
      egui::TextEdit::singleline(&mut text)
        .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
        .frame(egui::Frame::default())
        .margin(egui::Margin::symmetric(4, 2)),
    );

    response.request_focus();

    // Update text in state
    if let Some(mut state) = world.get_resource_mut::<ExplorerEditingState>() {
      state.text = text.clone();
    }

    // Handle Enter to confirm, Escape to cancel
    if response.lost_focus() {
      let submitted = ui.input(|i| i.key_pressed(egui::Key::Enter));
      let cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));

      if submitted && !text.is_empty() {
        let (mode, parent_path) = world
          .get_resource::<ExplorerEditingState>()
          .map(|state| (state.mode, state.parent_path.clone()))
          .unwrap_or((None, None));

        if let Some(parent) = parent_path {
          match mode {
            Some(ExplorerEditingMode::NewFile) => {
              world.spawn(CreateFileRequest::new(parent, text.clone()));
            }
            Some(ExplorerEditingMode::NewFolder) => {
              world.spawn(CreateFolderRequest::new(parent, text.clone()));
            }
            _ => {}
          }
        }
      }

      // Clear editing state
      if (submitted || cancelled)
        && let Some(mut state) =
          world.get_resource_mut::<ExplorerEditingState>()
      {
        state.mode = None;
      }
    }
  }
}

pub mod open_project {
  use crate::components::atoms::stripe_button;

  use codelord_core::dialog;
  use codelord_core::ecs::world::World;
  use codelord_core::navigation::resources::PendingFolderDialog;
  use codelord_core::runtime::RuntimeHandle;

  use eframe::egui;

  pub fn show(ui: &mut egui::Ui, world: &mut World) {
    let runtime = world.get_resource::<RuntimeHandle>().cloned();

    ui.vertical_centered(|ui| {
      ui.add_space(20.0);

      let button_response =
        stripe_button::show(ui, world, "OPEN PROJECT", egui::vec2(140.0, 30.0));

      if button_response.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
      }

      let has_pending = world.get_resource::<PendingFolderDialog>().is_some();
      let has_runtime = runtime.is_some();

      if button_response.clicked()
        && !has_pending
        && has_runtime
        && let Some(ref rt) = runtime
      {
        let rx = dialog::pick_folder(rt);
        world.insert_resource(PendingFolderDialog(rx));
      }

      ui.add_space(8.0);

      ui.label(
        egui::RichText::new("This folder is empty or all files are hidden")
          .color(ui.visuals().weak_text_color())
          .size(10.0),
      );
    });
  }
}
