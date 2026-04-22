use crate::components::atoms::{icon_button, stripe_button};
use crate::components::indicators::{feedback, metric};
use crate::components::navigation::{stagebar, tabbar};
use crate::components::structure::divider;
use crate::components::structure::divider::{Axis, LabelAlign};
use crate::components::views::{compiler_output, editor_content};

use codelord_core::animation::resources::ActiveAnimations;
use codelord_core::ecs::prelude::With;
use codelord_core::ecs::world::World;
use codelord_core::events::CompileRequest;
use codelord_core::icon::components::Icon;
use codelord_core::navigation::PlaygroundMode;
use codelord_core::navigation::resources::StagebarResource;
use codelord_core::playground::{
  FeedbackState, OutputViewKind, PlaygroundFeedback, PlaygroundMetrics,
  PlaygroundOutput, PlaygroundWebviewState, TemplatingTarget,
};
use codelord_core::previews::WebViewRect;
use codelord_core::tabbar::PlaygroundTab;
use codelord_core::text_editor::components::TextBuffer;
use codelord_core::ui::component::{Active, Metric};

use zo_runtime_native::renderer::Renderer;
use zo_runtime_render::render::Render;
use zo_ui_protocol::UiCommand;

use eframe::egui;

/// Two-column playground layout.
/// Left column: editor with tabbar (50% width).
/// Right column: output placeholder (50% width).
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let visuals = ui.style().visuals.clone();
  let separator_color = visuals.widgets.noninteractive.bg_stroke.color;

  // Tabbar panel at the top.
  egui::Panel::top("playground_tabbar")
    .frame(egui::Frame::NONE.fill(visuals.window_fill))
    .exact_size(24.0)
    .resizable(false)
    .show_separator_line(true)
    .show_inside(ui, |ui| tabbar::show::<PlaygroundTab>(ui, world));

  // Main content area with two columns.
  egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(visuals.window_fill))
    .show_inside(ui, |ui| {
      let content_rect = ui.available_rect_before_wrap();
      let half_width = content_rect.width() / 2.0;
      let separator_x = content_rect.left() + half_width;

      // Draw vertical separator at center.
      ui.painter().line_segment(
        [
          egui::pos2(separator_x, content_rect.top()),
          egui::pos2(separator_x, content_rect.bottom()),
        ],
        egui::Stroke::new(1.0_f32, separator_color),
      );

      // Left column: editor (50% width).
      let left_rect = egui::Rect::from_min_size(
        content_rect.min,
        egui::vec2(half_width, content_rect.height()),
      );
      ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
        editor_content::show::<PlaygroundTab>(ui, world, "playground_editor");
      });

      // Right column: output placeholder (50% width).
      let right_rect = egui::Rect::from_min_size(
        egui::pos2(separator_x + 1.0, content_rect.top()),
        egui::vec2(half_width - 1.0, content_rect.height()),
      );
      ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
        show_output_column(ui, world);
      });
    });
}

/// Right column: stagebar + metrics display.
fn show_output_column(ui: &mut egui::Ui, world: &mut World) {
  let visuals = ui.style().visuals.clone();

  ui.painter()
    .rect_filled(ui.max_rect(), 0.0, visuals.extreme_bg_color);

  // Get metric entities from resource
  let (output_entity, time_entity) = world
    .get_resource::<PlaygroundMetrics>()
    .map(|m| (m.output, m.time))
    .unwrap_or((None, None));

  // Get selected stage, mode, and check if it changed.
  let (selected_stage, stage_changed, mode) = world
    .get_resource::<StagebarResource>()
    .map(|s| (s.selected, s.is_animating && s.progress < 0.01, s.mode))
    .unwrap_or((0, false, PlaygroundMode::Programming));

  // Reset feedback to Ready when stage changes.
  if stage_changed
    && let Some(mut feedback) = world.get_resource_mut::<PlaygroundFeedback>()
  {
    feedback.state = FeedbackState::Ready;
  }

  // Sync metrics from compilation state.
  let (stage_count, elapsed_time) = world
    .get_resource::<PlaygroundOutput>()
    .map(|o| {
      let count = match (selected_stage, mode) {
        (0, _) => o.compilation.token_count,
        (1, _) => o.compilation.node_count,
        (2, _) => o.compilation.insn_count,
        (3, PlaygroundMode::Programming) => o.compilation.asm_bytes,
        (3, PlaygroundMode::Templating) => o.compilation.ui_count,
        _ => o.compilation.token_count,
      };

      (count, o.compilation.elapsed_time)
    })
    .unwrap_or((0, 0.0));

  // Sync count to output metric.
  if let Some(entity) = output_entity
    && let Some(mut metric) = world.get_mut::<Metric>(entity)
  {
    // Update unit based on selected stage and mode.
    metric.unit = match (selected_stage, mode) {
      (0, _) => "tokens",
      (1, _) => "nodes",
      (2, _) => "insns",
      (3, PlaygroundMode::Programming) => "bytes",
      (3, PlaygroundMode::Templating) => "cmds",
      _ => "tokens",
    };

    let target = metric.animation.target as usize;
    if target != stage_count {
      let started = metric.set_value(stage_count as f64);

      if started
        && let Some(mut active) = world.get_resource_mut::<ActiveAnimations>()
      {
        active.increment();
      }
    }
  }

  // Sync elapsed time to time metric.
  if let Some(entity) = time_entity
    && let Some(mut metric) = world.get_mut::<Metric>(entity)
  {
    let target = metric.animation.target;
    if (target - elapsed_time).abs() > 0.0001 {
      let started = metric.set_value(elapsed_time);

      if started
        && let Some(mut active) = world.get_resource_mut::<ActiveAnimations>()
      {
        active.increment();
      }
    }
  }

  ui.add_space(8.0);
  ui.vertical(|ui| {
    // Stagebar and Run button row
    ui.horizontal(|ui| {
      ui.add_space(8.0);
      stagebar::show(ui, world);

      // Web/Native checkbox (only in Templating mode)
      if mode == PlaygroundMode::Templating {
        ui.add_space(8.0);

        let mut is_web = world
          .get_resource::<PlaygroundOutput>()
          .map(|o| o.templating_target == TemplatingTarget::Web)
          .unwrap_or(true);

        let label = if is_web { "web" } else { "native" };
        let checkbox = egui::Checkbox::new(&mut is_web, label);

        if ui.add(checkbox).changed()
          && let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>()
        {
          output.templating_target = if is_web {
            TemplatingTarget::Web
          } else {
            TemplatingTarget::Native
          };
        }
      }

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        let response =
          stripe_button::show(ui, world, "Run", egui::vec2(80.0, 32.0));

        if response.clicked() {
          // Reset feedback state.
          if let Some(mut feedback) =
            world.get_resource_mut::<PlaygroundFeedback>()
          {
            feedback.state = FeedbackState::Ready;
          }
          // Get source code from active playground tab.
          let source = get_active_source(world);
          if !source.is_empty() {
            use codelord_protocol::compilation::Stage;

            // Map stage index to protocol stage. In Templating mode, slot
            // 3 means Ui (HTML output), not Asm.
            let protocol_stage = match (selected_stage, mode) {
              (0, _) => Stage::Tokens,
              (1, _) => Stage::Tree,
              (2, _) => Stage::Sir,
              (3, PlaygroundMode::Programming) => Stage::Asm,
              (3, PlaygroundMode::Templating) => Stage::Ui,
              _ => Stage::Tokens,
            };
            world.spawn(CompileRequest::new(source, "native", protocol_stage));
          }
        }
      });
    });

    ui.add_space(8.0);
    divider::show(ui, Axis::Horizontal);

    ui.vertical(|ui| {
      ui.add_space(8.0);

      ui.columns(2, |columns| {
        if let Some(entity) = output_entity {
          metric::show(&mut columns[0], world, entity);
        }
        if let Some(entity) = time_entity {
          metric::show(&mut columns[1], world, entity);
        }
      });
    });

    divider::show(ui, Axis::Horizontal);

    ui.horizontal(|ui| {
      ui.set_height(24.0);

      ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        feedback::show(ui, world);
      });

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);

        let active_view = world
          .get_resource::<PlaygroundOutput>()
          .map(|o| o.active_view)
          .unwrap_or_default();

        let text_color = ui.visuals().text_color();
        let weak_color = ui.visuals().weak_text_color();
        let size = egui::vec2(16.0, 16.0);

        // Browser button (webview) - Templating mode.
        let browser_color = if active_view == OutputViewKind::Webview {
          text_color
        } else {
          weak_color
        };
        if icon_button::show_sized(ui, &Icon::Browser, browser_color, size) {
          if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>()
          {
            output.active_view = OutputViewKind::Webview;
          }
          if let Some(mut stagebar) =
            world.get_resource_mut::<StagebarResource>()
          {
            stagebar.set_mode(PlaygroundMode::Templating);
          }
        }

        // Terminal button (compiler output) - Programming mode.
        let terminal_color = if active_view == OutputViewKind::CompilerOutput {
          text_color
        } else {
          weak_color
        };
        if icon_button::show_sized(ui, &Icon::Terminal, terminal_color, size) {
          if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>()
          {
            output.active_view = OutputViewKind::CompilerOutput;
          }
          if let Some(mut stagebar) =
            world.get_resource_mut::<StagebarResource>()
          {
            stagebar.set_mode(PlaygroundMode::Programming);
          }
        }
      });
    });

    // Get templating target (web vs native)
    let templating_target = world
      .get_resource::<PlaygroundOutput>()
      .map(|o| o.templating_target)
      .unwrap_or_default();

    // Determine what to display based on mode, stage, and target.
    // In Templating mode at stage 3:
    //   - Web target → show webview
    //   - Native target → show native egui view (TODO)
    // Otherwise show compiler output.
    let should_show_webview = mode == PlaygroundMode::Templating
      && selected_stage == 3
      && templating_target == TemplatingTarget::Web;

    let should_show_native = mode == PlaygroundMode::Templating
      && selected_stage == 3
      && templating_target == TemplatingTarget::Native;

    let label = if should_show_webview {
      "WEBViEW"
    } else if should_show_native {
      "NATiVE"
    } else {
      match (selected_stage, mode) {
        (0, _) => "LEXiCAL ANALYSiS",
        (1, _) => "SYNTAX ANALYSiS",
        (2, _) => "SEMANTiC iR",
        (3, PlaygroundMode::Programming) => "ASSEMBLY CODE",
        (3, PlaygroundMode::Templating) => "Ui COMMANDS",
        (4, _) => "HEXADECIMAL DUMP",
        _ => "COMPiLER OUTPUT",
      }
    };

    divider::show_with_label(ui, label, LabelAlign::Center);

    // Display the appropriate view.
    if should_show_webview {
      // Reserve space for the webview and update ECS state
      let available = ui.available_rect_before_wrap();

      // Update webview state in ECS (use logical coordinates like HTML preview)
      if let Some(mut webview_state) =
        world.get_resource_mut::<PlaygroundWebviewState>()
      {
        webview_state.enabled = true;
        webview_state.webview_rect = Some(WebViewRect {
          x: available.min.x as f64,
          y: available.min.y as f64,
          width: available.width() as f64,
          height: available.height() as f64,
        });
      }

      // Reserve the space (webview is rendered by Coder::update)
      ui.allocate_rect(available, egui::Sense::hover());
    } else if should_show_native {
      // Disable webview when showing native
      if let Some(mut webview_state) =
        world.get_resource_mut::<PlaygroundWebviewState>()
      {
        webview_state.enabled = false;
      }

      // Get UI commands JSON from compilation output
      let ui_json = world
        .get_resource::<PlaygroundOutput>()
        .and_then(|o| o.compilation.ui.clone());

      if let Some(json) = ui_json {
        // Render native egui view using zo-runtime-native
        render_native_ui(ui, &json);
      } else {
        ui.vertical_centered(|ui| {
          ui.add_space(20.0);
          ui.label(
            egui::RichText::new("No UI commands available.")
              .color(ui.visuals().weak_text_color())
              .size(12.0),
          );
          ui.label(
            egui::RichText::new("Run compilation to see native UI preview.")
              .color(ui.visuals().weak_text_color())
              .size(11.0),
          );
        });
      }
    } else {
      // Disable playground webview when showing compiler output
      if let Some(mut webview_state) =
        world.get_resource_mut::<PlaygroundWebviewState>()
      {
        webview_state.enabled = false;
      }

      compiler_output::show(ui, world);
    }
  });
}

/// Get source code from active playground tab.
fn get_active_source(world: &mut World) -> String {
  world
    .query_filtered::<&TextBuffer, (With<PlaygroundTab>, With<Active>)>()
    .iter(world)
    .next()
    .map(|buffer: &TextBuffer| buffer.rope.to_string())
    .unwrap_or_default()
}

/// JSON structure for UI output from server.
#[derive(serde::Deserialize)]
struct UiOutput {
  commands: Vec<UiCommand>,
}

/// Render native egui UI from UiCommands JSON.
fn render_native_ui(ui: &mut egui::Ui, json: &str) {
  match sonic_rs::from_str::<UiOutput>(json) {
    Ok(output) => {
      if output.commands.is_empty() {
        ui.vertical_centered(|ui| {
          ui.add_space(20.0);
          ui.label(
            egui::RichText::new("No UI commands to render.")
              .color(ui.visuals().weak_text_color())
              .size(12.0),
          );
        });

        return;
      }

      // Create renderer and render commands
      let mut renderer = Renderer::new();
      renderer.render(&output.commands);
      renderer.render_with_ui(ui);
    }
    Err(e) => {
      ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(
          egui::RichText::new("Failed to parse UI commands.")
            .color(ui.visuals().error_fg_color)
            .size(12.0),
        );
        ui.label(
          egui::RichText::new(format!("{e}"))
            .color(ui.visuals().weak_text_color())
            .size(10.0),
        );
      });
    }
  }
}
