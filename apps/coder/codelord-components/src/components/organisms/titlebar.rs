use crate::assets::icon;
use crate::components::atoms::decoration;
use crate::components::atoms::decoration::DecorationStyle;
use crate::components::structure::divider;
use crate::components::structure::divider::Axis;
use crate::components::voice_visualizer;

use codelord_core::drag_and_drop::{
  DragAnimationState, DragAxis, DragOrder, DragState,
};
use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::world::World;
use codelord_core::git::resources::GitBranchState;
use codelord_core::icon::components::{Dot, Icon, TitlebarIcon};
use codelord_core::navigation::resources::ActiveWorkspaceRoot;
use codelord_core::page::components::Page;
use codelord_core::page::resources::{PageResource, PageSwitchCommand};
use codelord_core::popup::resources::{
  PopupAction, PopupCommand, PopupResource,
};
use codelord_core::ui::component::DecorationType;

use eframe::egui;

/// Render the titlebar
///
/// Shows menu bar, file name, and window controls.
/// Reads state from ECS World but doesn't mutate it.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  ui.horizontal_centered(|ui| {
    let app_rect = ui.max_rect();
    let title_bar_height = 28.0;

    let title_bar_rect = {
      let mut rect = app_rect;
      rect.max.y = rect.min.y + title_bar_height;
      rect
    };

    let title_bar_response = ui.interact(
      title_bar_rect,
      egui::Id::new("title_bar_overlay"),
      egui::Sense::click_and_drag(),
    );

    if title_bar_response.drag_started_by(egui::PointerButton::Primary) {
      ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    if title_bar_response.double_clicked() {
      let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));

      ui.ctx()
        .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
    }

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      ui.spacing_mut().item_spacing.x = 8.0;
      ui.add_space(8.0);

      // Query decoration entities
      let mut query = world.query::<(
        codelord_core::ecs::entity::Entity,
        &codelord_core::ui::component::DecorationType,
      )>();

      for (entity, decoration_type) in query.iter(world) {
        let style = match decoration_type {
          DecorationType::Close => DecorationStyle::close(),
          DecorationType::MinimizeMaximize => DecorationStyle::minimize(),
          DecorationType::Fullscreen => DecorationStyle::maximize(),
        };

        let clicked = decoration::show(ui, world, entity, style);

        // Handle decoration click based on type
        match decoration_type {
          DecorationType::Close => {
            if clicked {
              ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
          }
          DecorationType::MinimizeMaximize => {
            let is_minimized =
              ui.input(|i| i.viewport().minimized.unwrap_or(false));
            if is_minimized {
              ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            } else if clicked {
              ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
          }
          DecorationType::Fullscreen => {
            if clicked {
              let is_fullscreen =
                ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
              ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                  !is_fullscreen,
                ));
            }
          }
        }
      }

      // Query titlebar icons with entity and order
      let mut icon_query =
        world.query::<(Entity, &Icon, &TitlebarIcon, &DragOrder)>();
      let mut icons: Vec<_> = icon_query
        .iter(world)
        .map(|(e, i, _, o)| (e, *i, o.0))
        .collect();
      icons.sort_by_key(|(_, _, order)| *order);

      let active_page = world
        .get_resource::<PageResource>()
        .map(|r| r.active_page)
        .unwrap_or(Page::Welcome);

      // Get drag state
      let drag_state = world.get_resource::<DragState>();
      let dragging =
        drag_state.and_then(|s| s.payload::<TitlebarDragPayload>().cloned());
      let drag_delta_x = drag_state.map(|s| s.delta()[0]).unwrap_or(0.0);

      // Get delta time for animation
      let dt = ui.input(|i| i.stable_dt);

      // Icon dimensions
      const ICON_SIZE: f32 = 24.0;
      let total_width = ICON_SIZE * icons.len() as f32;

      // Reserve fixed space for icons area
      let (icon_area_rect, _) = ui.allocate_exact_size(
        egui::vec2(total_width, ICON_SIZE),
        egui::Sense::hover(),
      );
      let start_x = icon_area_rect.min.x;
      let center_y = icon_area_rect.center().y;

      // Calculate dragged icon's visual position
      let dragged_visual_index = if let Some(ref d) = dragging {
        let dragged_order = d.order as f32;
        let new_pos = dragged_order + drag_delta_x / ICON_SIZE;
        Some(new_pos.clamp(0.0, (icons.len() - 1) as f32))
      } else {
        None
      };

      // First pass: calculate target positions and collect render data
      let mut render_data: Vec<(Entity, Icon, u32, Page, f32, bool)> =
        Vec::new();

      for (entity, icon_component, order) in &icons {
        let page = match icon_component {
          Icon::Home => Page::Welcome,
          Icon::Code => Page::Editor,
          Icon::Ufo => Page::Playground,
          Icon::Alien => Page::Presenter,
          _ => continue,
        };

        let is_being_dragged = dragging
          .as_ref()
          .map(|d| d.entity == *entity)
          .unwrap_or(false);

        // Calculate target visual position
        let target_idx = if is_being_dragged {
          dragged_visual_index.unwrap_or(*order as f32)
        } else if let Some(dragged_pos) = dragged_visual_index {
          if let Some(ref d) = dragging {
            let my_order = *order as f32;
            let orig_dragged_order = d.order as f32;
            if orig_dragged_order < my_order && dragged_pos >= my_order {
              my_order - 1.0
            } else if orig_dragged_order > my_order && dragged_pos <= my_order {
              my_order + 1.0
            } else {
              my_order
            }
          } else {
            *order as f32
          }
        } else {
          *order as f32
        };

        render_data.push((
          *entity,
          *icon_component,
          *order,
          page,
          target_idx,
          is_being_dragged,
        ));
      }

      // Second pass: animate and render
      for (entity, icon_component, order, page, target_idx, is_being_dragged) in
        &render_data
      {
        // Animate position (dragged icon follows directly, others animate)
        let visual_x = if *is_being_dragged {
          start_x + target_idx * ICON_SIZE
        } else {
          let key = format!("titlebar_{}", entity.index());
          let target_x = start_x + target_idx * ICON_SIZE;
          if let Some(mut anim) = world.get_resource_mut::<DragAnimationState>()
          {
            anim.animate_to(&key, target_x, dt)
          } else {
            target_x
          }
        };

        let icon_rect = egui::Rect::from_center_size(
          egui::pos2(visual_x + ICON_SIZE / 2.0, center_y),
          egui::vec2(ICON_SIZE, ICON_SIZE),
        );

        let is_active = *page == active_page;
        let tint = if is_active {
          ui.style().visuals.widgets.hovered.fg_stroke.color
        } else {
          ui.style().visuals.text_color()
        };

        // Interact with icon (no space allocation - already reserved)
        let response = ui.interact(
          icon_rect,
          egui::Id::new(("titlebar_icon", entity.index())),
          egui::Sense::click_and_drag(),
        );

        // Paint icon directly without affecting layout
        let image = icon::icon_to_image(icon_component)
          .fit_to_exact_size(egui::vec2(16.0, 16.0))
          .tint(tint);
        image.paint_at(ui, icon_rect.shrink(4.0));

        // Handle drag start
        if response.drag_started()
          && let Some(pos) = ui.ctx().pointer_hover_pos()
          && let Some(mut state) = world.get_resource_mut::<DragState>()
        {
          state.set_payload(
            TitlebarDragPayload {
              entity: *entity,
              icon: *icon_component,
              order: *order,
            },
            DragAxis::X,
            "titlebar",
            [pos.x, pos.y],
          );
        }

        // Update drag position
        if response.dragged() {
          if let Some(pos) = ui.ctx().pointer_hover_pos()
            && let Some(mut state) = world.get_resource_mut::<DragState>()
            && state.has_payload::<TitlebarDragPayload>()
          {
            state.update_pos([pos.x, pos.y]);
          }
          ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        // Handle drop - reorder on release
        if response.drag_stopped() {
          if let Some(ref d) = dragging
            && d.entity == *entity
          {
            let new_order =
              dragged_visual_index.unwrap_or(*order as f32).round() as u32;

            if new_order != d.order {
              for (other_entity, _, other_order, _, _, _) in &render_data {
                if *other_entity == d.entity {
                  continue;
                }
                let other = *other_order;
                let orig = d.order;
                let target = new_order;

                let new_other_order = if orig < target {
                  if other > orig && other <= target {
                    other - 1
                  } else {
                    other
                  }
                } else if other >= target && other < orig {
                  other + 1
                } else {
                  other
                };

                if new_other_order != other
                  && let Some(mut order_comp) =
                    world.get_mut::<DragOrder>(*other_entity)
                {
                  order_comp.0 = new_other_order;
                }
              }

              if let Some(mut order_comp) = world.get_mut::<DragOrder>(d.entity)
              {
                order_comp.0 = new_order;
              }
            }
          }

          if let Some(mut state) = world.get_resource_mut::<DragState>() {
            state.clear();
          }
        }

        // Handle click
        if response.clicked() {
          world.write_message(PageSwitchCommand { page: *page });
        }

        if response.hovered() && dragging.is_none() {
          ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }
      }

      ui.add_space(-4.0); // Reduce spacing before divider
      divider::show(ui, Axis::Vertical);

      // Display workspace name and git branch as separate buttons
      let workspace_path = world
        .get_resource::<ActiveWorkspaceRoot>()
        .and_then(|r| r.path.clone());

      let workspace_name = world
        .get_resource::<ActiveWorkspaceRoot>()
        .and_then(|r| r.name.clone());

      // Only show branch if it belongs to the current workspace
      let (branch_info, is_dirty) = world
        .get_resource::<GitBranchState>()
        .map_or((None, false), |git| {
          // Check if branch state matches current workspace
          if git.workspace_path == workspace_path && !git.loading {
            (git.branch.clone(), git.is_dirty)
          } else {
            (None, false)
          }
        });

      let text_color = ui.style().visuals.text_color();
      // Gray color for branch
      let gray_color = egui::Color32::from_gray(130);

      if let Some(name) = workspace_name {
        // Workspace button
        let workspace_btn = ui.add(
          egui::Button::new(
            egui::RichText::new(&name).color(text_color).size(12.0),
          )
          .frame(false),
        );

        if workspace_btn.hovered() {
          ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }

        // Show branch if available and matches current workspace
        if let Some(branch) = branch_info {
          // Separator
          ui.label(egui::RichText::new("—").color(gray_color).size(12.0));

          // Branch with dirty indicator
          let branch_display = if is_dirty {
            format!("{branch}*")
          } else {
            branch
          };

          // Branch button (gray color)
          let branch_btn = ui.add(
            egui::Button::new(
              egui::RichText::new(&branch_display)
                .color(gray_color)
                .size(12.0),
            )
            .frame(false),
          );

          if branch_btn.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
          }
        }
      }
    });

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.spacing_mut().item_spacing.x = 8.0;
      ui.add_space(8.0);

      let settings_response = ui.add(
        egui::Button::image(
          icon::icon_to_image(&Icon::Dot(Dot::Horizontal))
            .fit_to_exact_size(egui::Vec2::splat(16.0))
            .tint(ui.style().visuals.text_color()),
        )
        .frame(false),
      );

      if settings_response.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
      }

      settings_response.clicked().then(|| {
        world
          .get_resource::<PopupResource>()
          .and_then(|r| r.settings_popup)
          .map(|popup_entity| {
            let rect = settings_response.rect;

            let anchor_rect =
              [rect.min.x, rect.max.y, rect.width(), rect.height()];

            world.write_message(PopupCommand {
              action: PopupAction::Toggle {
                entity: popup_entity,
                anchor_rect,
              },
            })
          })
      });

      voice_visualizer::show(ui, world);
    });
  });
}

/// Payload for dragging titlebar icons.
#[derive(Clone)]
struct TitlebarDragPayload {
  entity: Entity,
  #[allow(dead_code)]
  icon: Icon,
  #[allow(dead_code)]
  order: u32,
}
