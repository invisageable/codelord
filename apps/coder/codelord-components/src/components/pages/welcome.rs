use crate::assets::font;
use crate::assets::theme::get_theme;
use crate::components::effects::{shimmer, wave};
use crate::components::navigation::xmb;

use codelord_core::ecs::world::World;
use codelord_core::xmb::resources::XmbResource;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let time = ui.ctx().input(|i| i.time);
  let rect = ui.max_rect();

  // Get theme text color
  let theme = get_theme(world);
  let text_color = egui::Color32::from_rgba_unmultiplied(
    theme.text[0],
    theme.text[1],
    theme.text[2],
    theme.text[3],
  );

  wave::show(ui, rect, time as f32, world);

  // Overlay layer with opacity between wave and content
  ui.painter().rect_filled(
    rect,
    egui::CornerRadius::ZERO,
    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120),
  );

  // Two-column layout: logo on left, XMB on right
  ui.columns(2, |columns| {
    // Left column: logo and info
    columns[0].vertical_centered_justified(|ui| {
      ui.horizontal(|ui| {
        ui.add_space(60.0);
        ui.vertical(|ui| {
          ui.add_space(60.0);
          render_logo(ui, text_color);
          render_tagline(ui, world, time as f32);
          ui.add_space(40.0);
          render_description(ui, world, text_color);
        });
      });
    });

    // Right column: XMB navigation
    let xmb_rect = columns[1].available_rect_before_wrap();
    xmb::show(&mut columns[1], world, xmb_rect);
  });
}

/// Render the codelord logo.
fn render_logo(ui: &mut egui::Ui, text_color: egui::Color32) {
  ui.label(
    egui::RichText::new("WELCOME TO")
      .font(egui::FontId::new(
        48.0,
        egui::FontFamily::Name(font::SUISSE_INTL.into()),
      ))
      .color(text_color)
      .extra_letter_spacing(2.0)
      .line_height(Some(32.0)),
  );

  let mut layout_job = egui::text::LayoutJob::default();
  layout_job.append(
    "code",
    0.0,
    egui::TextFormat {
      font_id: egui::FontId::new(
        90.0,
        egui::FontFamily::Name(font::AEONIK.into()),
      ),
      color: text_color,
      extra_letter_spacing: -4.0,
      ..Default::default()
    },
  );
  layout_job.append(
    "lord",
    0.0,
    egui::TextFormat {
      font_id: egui::FontId::new(
        100.0,
        egui::FontFamily::Name(font::CIRKA.into()),
      ),
      color: text_color,
      extra_letter_spacing: -4.0,
      line_height: Some(118.0),
      ..Default::default()
    },
  );
  ui.label(layout_job);
}

/// Render the tagline with shimmer effect.
fn render_tagline(ui: &mut egui::Ui, world: &mut World, time: f32) {
  shimmer::show(ui, world, "JOiN THE DEVOLUTiON", 33.0, time);
}

/// Render the focused item description with hacker animation.
fn render_description(
  ui: &mut egui::Ui,
  world: &World,
  text_color: egui::Color32,
) {
  if let Some(description_text) = world
    .get_resource::<XmbResource>()
    .and_then(|xmb| xmb.description_text())
  {
    ui.label(
      egui::RichText::new(description_text)
        .color(text_color)
        .size(14.0),
    );
  }
}
