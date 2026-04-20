//! Popup overlay component for rendering popup menus.
//!
//! Pure UI component that reads popup state from ECS World.

use crate::assets::theme::get_theme;

use codelord_core::ecs::entity::Entity;
use codelord_core::ecs::prelude::With;
use codelord_core::ecs::world::World;
use codelord_core::events::{
  AddFolderToWorkspaceDialogRequest, CloseAllTabsRequest,
  CloseOtherTabsRequest, CloseTabRequest, CloseTabsToRightRequest,
  DeleteRequest, PasteRequest, RemoveRootRequest,
};
use codelord_core::navigation::resources::{
  ExplorerContextTarget, ExplorerEditingMode, ExplorerEditingState,
  ExplorerState, FileClipboard,
};
use codelord_core::page::components::Page;
use codelord_core::page::resources::PageSwitchCommand;
use codelord_core::popup::components::{
  MenuItem, Popup, PopupContent, PopupPosition, PopupVisible,
};
use codelord_core::popup::resources::{
  PopupAction, PopupCommand, PopupResource,
};
use codelord_core::previews::sqlite::ExportRequest;
use codelord_core::tabbar::TabContextTarget;
use codelord_core::toast::resources::ToastCommand;

use eframe::egui;

/// Render all visible popups as overlays.
pub fn show(ctx: &egui::Context, world: &mut World) {
  // Get button hover colors from theme
  let theme = get_theme(world);
  let hover_bg = egui::Color32::from_rgba_unmultiplied(
    theme.button_hover_bg[0],
    theme.button_hover_bg[1],
    theme.button_hover_bg[2],
    theme.button_hover_bg[3],
  );
  let hover_fg = egui::Color32::from_rgba_unmultiplied(
    theme.button_hover_fg[0],
    theme.button_hover_fg[1],
    theme.button_hover_fg[2],
    theme.button_hover_fg[3],
  );

  // Collect visible popups
  let popups: Vec<(Entity, Popup)> = world
    .query_filtered::<(Entity, &Popup), With<PopupVisible>>()
    .iter(world)
    .map(|(e, p)| (e, p.clone()))
    .collect();

  // Track if we need to close popups
  let mut close_popup: Option<Entity> = None;
  let mut selected_item: Option<(Entity, String)> = None;

  for (entity, popup) in &popups {
    if let Some(anchor_rect) = popup.anchor_rect {
      let [x, y, _w, _h] = anchor_rect;
      let position = calculate_popup_position(ctx, popup.position, x, y);

      egui::Area::new(egui::Id::new(format!("popup_{entity:?}")))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
          ui.set_max_width(256.0);
          ui.set_min_width(256.0);

          let visuals = ui.visuals();
          let bg_color = visuals.window_fill;
          let stroke_color = visuals.widgets.noninteractive.bg_stroke.color;

          egui::Frame::NONE
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
            .outer_margin(egui::Margin::same(8))
            .corner_radius(egui::CornerRadius::ZERO)
            .shadow(egui::epaint::Shadow {
              offset: [2, 2],
              blur: 8,
              spread: 0,
              color: egui::Color32::from_black_alpha(128),
            })
            .show(ui, |ui| {
              if let Some(item_id) =
                render_popup_content(ui, &popup.content, hover_bg, hover_fg)
              {
                selected_item = Some((*entity, item_id));
              }
            });
        });
    }
  }

  // Handle click outside to close popups
  ctx.input(|i| {
    if i.pointer.primary_clicked()
      && let Some(pos) = i.pointer.interact_pos()
    {
      let any_visible = world
        .get_resource::<PopupResource>()
        .map(|r| r.active_popup.is_some())
        .unwrap_or(false);

      if any_visible {
        // Check if click is outside all visible popups
        let click_inside = popups.iter().any(|(_, popup)| {
          popup
            .anchor_rect
            .map(|[x, y, _w, h]| {
              // Approximate popup bounds (actual popup is 256px wide)
              let popup_x = x;
              let popup_y = y;
              let popup_w = 256.0 + 16.0; // width + outer margin
              let popup_h = h + 200.0; // approximate height

              pos.x >= popup_x
                && pos.x <= popup_x + popup_w
                && pos.y >= popup_y
                && pos.y <= popup_y + popup_h
            })
            .unwrap_or(false)
        });

        if !click_inside && !popups.is_empty() {
          close_popup = popups.first().map(|(e, _)| *e);
        }
      }
    }
  });

  // Process popup close
  close_popup.map(|entity| {
    world.write_message(PopupCommand {
      action: PopupAction::Hide(entity),
    })
  });

  // Process selected item - navigate to page and close popup
  selected_item.map(|(entity, item_id)| {
    // Handle navigation based on item id
    match item_id.as_str() {
      "settings" => {
        world.write_message(PageSwitchCommand {
          page: Page::Settings,
        });
      }
      "about" => {
        world.write_message(PageSwitchCommand { page: Page::About });
      }
      "check_updates" => {
        world.write_message(ToastCommand::info("No updates available"));
      }
      // Explorer context menu actions
      "new_file" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|target| target.path.clone())
        {
          // Get parent path for new file (if target is dir, use it; else use
          // parent)
          let parent_path = world
            .get_resource::<ExplorerContextTarget>()
            .and_then(|t| {
              t.is_dir
                .then(|| path.clone())
                .or_else(|| path.parent().map(|p| p.to_path_buf()))
            })
            .unwrap_or(path.clone());

          if let Some(mut state) =
            world.get_resource_mut::<ExplorerEditingState>()
          {
            state.mode = Some(ExplorerEditingMode::NewFile);
            state.text = String::new();
            state.target_entity = None;
            state.parent_path = Some(parent_path);
            state.depth = 1; // Will be calculated in explorer.rs
          }
        }
      }
      "new_folder" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|target| target.path.clone())
        {
          let parent_path = world
            .get_resource::<ExplorerContextTarget>()
            .and_then(|t| {
              t.is_dir
                .then(|| path.clone())
                .or_else(|| path.parent().map(|p| p.to_path_buf()))
            })
            .unwrap_or(path.clone());

          if let Some(mut state) =
            world.get_resource_mut::<ExplorerEditingState>()
          {
            state.mode = Some(ExplorerEditingMode::NewFolder);
            state.text = String::new();
            state.target_entity = None;
            state.parent_path = Some(parent_path);
            state.depth = 1;
          }
        }
      }
      "rename" => {
        let target_info = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|target| target.entity.zip(target.path.clone()));

        if let Some((e, p)) = target_info {
          let current_name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

          if let Some(mut state) =
            world.get_resource_mut::<ExplorerEditingState>()
          {
            state.mode = Some(ExplorerEditingMode::Rename);
            state.text = current_name;
            state.target_entity = Some(e);
            state.parent_path = p.parent().map(|p| p.to_path_buf());
            state.depth = 1;
          }
        }
      }
      "delete" => {
        let target_info = world
          .get_resource::<ExplorerContextTarget>()
          .map(|target| (target.entity, target.path.clone(), target.is_dir));

        target_info.and_then(|(e, p, is_dir)| {
          e.zip(p).map(|(entity, path)| {
            world.spawn(DeleteRequest::new(entity, path, is_dir));
          })
        });
      }
      "copy_path" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|target| target.path.clone())
        {
          ctx.copy_text(path.display().to_string());
          world
            .write_message(ToastCommand::success("Path copied to clipboard"));
        }
      }
      "copy_relative_path" => {
        let (target_path, roots) = (
          world
            .get_resource::<ExplorerContextTarget>()
            .and_then(|t| t.path.clone()),
          world
            .get_resource::<ExplorerState>()
            .map(|s| s.roots.clone())
            .unwrap_or_default(),
        );

        if let Some(path) = target_path {
          // Try to find a matching root
          let relative = roots
            .iter()
            .find_map(|root| {
              path
                .strip_prefix(root)
                .ok()
                .map(|p| p.display().to_string())
            })
            .unwrap_or_else(|| path.display().to_string());
          ctx.copy_text(relative);
          world.write_message(ToastCommand::success(
            "Relative path copied to clipboard",
          ));
        }
      }
      "reveal_in_finder" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|target| target.path.clone())
        {
          #[cfg(target_os = "macos")]
          {
            let _ = std::process::Command::new("open")
              .arg("-R")
              .arg(&path)
              .spawn();
          }
          #[cfg(target_os = "linux")]
          {
            let _ = std::process::Command::new("xdg-open")
              .arg(path.parent().unwrap_or(&path))
              .spawn();
          }
          #[cfg(target_os = "windows")]
          {
            let _ = std::process::Command::new("explorer")
              .arg("/select,")
              .arg(&path)
              .spawn();
          }
        }
      }
      "cut" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|t| t.path.clone())
        {
          if let Some(mut cb) = world.get_resource_mut::<FileClipboard>() {
            cb.set_cut(path);
          }

          world.write_message(ToastCommand::info("Cut to clipboard"));
        }
      }
      "copy" => {
        if let Some(path) = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|t| t.path.clone())
        {
          if let Some(mut cb) = world.get_resource_mut::<FileClipboard>() {
            cb.set_copy(path);
          }

          world.write_message(ToastCommand::info("Copied to clipboard"));
        }
      }
      "paste" => {
        let target_path = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|t| t.is_dir.then(|| t.path.clone()).flatten());

        let clipboard_data = world
          .get_resource::<FileClipboard>()
          .map(|cb| (cb.path.clone(), cb.is_cut));

        if let Some((dest, (Some(src), is_cut))) =
          target_path.zip(clipboard_data)
        {
          world.spawn(PasteRequest::new(src, dest, is_cut));
        }
      }
      "add_folder_to_workspace" => {
        // Spawn dialog request - the system will open the folder picker
        world.spawn(AddFolderToWorkspaceDialogRequest);
      }
      "remove_from_workspace" => {
        // Get context target path and check if it's a root
        let target_path = world
          .get_resource::<ExplorerContextTarget>()
          .and_then(|t| t.path.clone());

        let (is_root, can_remove) = world
          .get_resource::<ExplorerState>()
          .map(|s| {
            let is_root =
              target_path.as_ref().map(|p| s.is_root(p)).unwrap_or(false);
            let can_remove = s.is_multi_root();
            (is_root, can_remove)
          })
          .unwrap_or((false, false));

        if let Some(path) = target_path {
          if is_root && can_remove {
            world.spawn(RemoveRootRequest::new(path));
          } else if !can_remove {
            world.write_message(ToastCommand::warning(
              "Cannot remove last folder from workspace",
            ));
          } else {
            world.write_message(ToastCommand::warning(
              "Can only remove root folders from workspace",
            ));
          }
        }
      }
      // Tab context menu actions
      "close_tab" => {
        if let Some(tab_entity) = world
          .get_resource::<TabContextTarget>()
          .and_then(|t| t.entity)
        {
          world.spawn(CloseTabRequest::new(tab_entity));
        }
      }
      "close_others" => {
        if let Some(tab_entity) = world
          .get_resource::<TabContextTarget>()
          .and_then(|t| t.entity)
        {
          world.spawn(CloseOtherTabsRequest::new(tab_entity));
        }
      }
      "close_to_right" => {
        if let Some(tab_entity) = world
          .get_resource::<TabContextTarget>()
          .and_then(|t| t.entity)
        {
          world.spawn(CloseTabsToRightRequest::new(tab_entity));
        }
      }
      "close_all" => {
        world.spawn(CloseAllTabsRequest);
      }
      // SQLite export actions
      "export_csv" => {
        world.spawn(ExportRequest::Csv);
      }
      "export_json" => {
        world.spawn(ExportRequest::Json);
      }
      _ => {}
    }

    // Close the popup
    world.write_message(PopupCommand {
      action: PopupAction::Hide(entity),
    })
  });
}

/// Calculate the position for the popup based on its positioning mode.
fn calculate_popup_position(
  ctx: &egui::Context,
  position: PopupPosition,
  anchor_x: f32,
  anchor_y: f32,
) -> egui::Pos2 {
  let content_rect = ctx.input(|i| i.content_rect());
  let margin = 4.0;
  const POPUP_WIDTH: f32 = 256.0;

  match position {
    PopupPosition::Below => {
      let x = anchor_x.min(content_rect.max.x - POPUP_WIDTH);
      let y = (anchor_y + margin).min(content_rect.max.y - 300.0);
      egui::pos2(x, y)
    }
    PopupPosition::Above => {
      let x = anchor_x.min(content_rect.max.x - POPUP_WIDTH);
      let y = (anchor_y - 300.0 - margin).max(content_rect.min.y);
      egui::pos2(x, y)
    }
    PopupPosition::Right => {
      let x = (anchor_x + margin).min(content_rect.max.x - POPUP_WIDTH);
      let y = anchor_y.min(content_rect.max.y - 300.0);
      egui::pos2(x, y)
    }
    PopupPosition::Left => {
      let x = (anchor_x - POPUP_WIDTH - margin).max(content_rect.min.x);
      let y = anchor_y.min(content_rect.max.y - 300.0);
      egui::pos2(x, y)
    }
    PopupPosition::Cursor => {
      // Use the anchor position (stored when popup was shown), not current
      // mouse
      egui::pos2(
        anchor_x.min(content_rect.max.x - POPUP_WIDTH),
        anchor_y.min(content_rect.max.y - 300.0),
      )
    }
    PopupPosition::Absolute(x, y) => egui::pos2(x, y),
  }
}

/// Render the content of a popup. Returns selected item id if clicked.
fn render_popup_content(
  ui: &mut egui::Ui,
  content: &PopupContent,
  hover_bg: egui::Color32,
  hover_fg: egui::Color32,
) -> Option<String> {
  match content {
    PopupContent::Menu(items) => {
      render_menu_items(ui, items, hover_bg, hover_fg)
    }
    PopupContent::Custom(custom_id) => {
      render_custom_content(ui, custom_id);
      None
    }
  }
}

/// Render menu items. Returns selected item id if clicked.
fn render_menu_items(
  ui: &mut egui::Ui,
  items: &[MenuItem],
  hover_bg: egui::Color32,
  hover_fg: egui::Color32,
) -> Option<String> {
  ui.set_min_width(256.0);

  let mut selected = None;

  for (index, item) in items.iter().enumerate() {
    if index > 0 && items[index - 1].separator_after {
      ui.separator();
    }

    let response = ui.allocate_response(
      egui::vec2(ui.available_width(), 22.0),
      if item.enabled {
        egui::Sense::click()
      } else {
        egui::Sense::hover()
      },
    );

    let is_hovered = item.enabled
      && ui
        .input(|i| i.pointer.hover_pos())
        .map(|pos| response.rect.contains(pos))
        .unwrap_or(false);

    // Get theme colors
    let visuals = ui.visuals();
    let normal_text = visuals.text_color();
    let weak_text = visuals.weak_text_color();

    // Highlight on hover
    if is_hovered {
      ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
      ui.painter().rect_filled(response.rect, 0.0, hover_bg);
    }

    // Draw menu item content
    ui.scope_builder(egui::UiBuilder::new().max_rect(response.rect), |ui| {
      ui.add_enabled_ui(item.enabled, |ui| {
        ui.horizontal(|ui| {
          ui.set_height(22.0);

          ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
              ui.add_space(8.0);

              if let Some(icon) = &item.icon {
                ui.add(
                  egui::Label::new(
                    egui::RichText::new(icon).color(if is_hovered {
                      hover_fg
                    } else {
                      weak_text
                    }),
                  )
                  .selectable(false),
                );
              }

              ui.add(
                egui::Label::new(
                  egui::RichText::new(&item.label).color(if is_hovered {
                    hover_fg
                  } else {
                    normal_text
                  }),
                )
                .selectable(false),
              );
            },
          );

          if let Some(shortcut) = &item.shortcut {
            ui.with_layout(
              egui::Layout::right_to_left(egui::Align::Center),
              |ui| {
                ui.add_space(8.0);
                ui.add(
                  egui::Label::new(
                    egui::RichText::new(shortcut).size(10.0).color(weak_text),
                  )
                  .selectable(false),
                );
              },
            );
          }
        });
      });
    });

    if response.clicked() && item.enabled {
      selected = Some(item.id.clone());
    }
  }

  selected
}

/// Render custom popup content.
fn render_custom_content(ui: &mut egui::Ui, custom_id: &str) {
  ui.label(format!("Custom content: {custom_id}"));
}
