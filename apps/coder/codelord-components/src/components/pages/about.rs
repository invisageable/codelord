//! About page.

use crate::assets::font;
use crate::assets::theme::get_theme;

use codelord_core::about::resources::AboutResource;
use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  egui::ScrollArea::vertical()
    .id_salt("about_scrollbar")
    .show(ui, |ui| {
      let section_height = ui.available_height();

      render_header(ui, world, section_height);
      render_body(ui, world);
      render_footer(ui, world);
    });
}

fn render_header(ui: &mut egui::Ui, world: &World, section_height: f32) {
  let theme = get_theme(world);
  let bg_color = egui::Color32::from_rgba_unmultiplied(
    theme.secondary[0],
    theme.secondary[1],
    theme.secondary[2],
    theme.secondary[3],
  );
  let text_color = ui.style().visuals.text_color();

  let header_rect = ui
    .allocate_space(egui::vec2(ui.available_width(), section_height))
    .1;

  ui.painter()
    .rect_filled(header_rect, egui::CornerRadius::ZERO, bg_color);

  ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
    ui.centered_and_justified(|ui| {
      let mut layout_job = egui::text::LayoutJob::default();

      layout_job.append(
        "code",
        0.0,
        egui::TextFormat {
          font_id: font::aeonik(140.0),
          color: text_color,
          extra_letter_spacing: -4.0,
          ..Default::default()
        },
      );

      layout_job.append(
        "lord",
        0.0,
        egui::TextFormat {
          font_id: font::cirka(150.0),
          color: text_color,
          extra_letter_spacing: -4.0,
          line_height: Some(178.0),
          ..Default::default()
        },
      );

      ui.label(layout_job);
    });
  });
}

fn render_body(ui: &mut egui::Ui, world: &mut World) {
  let theme = get_theme(world);
  let primary_color = egui::Color32::from_rgba_unmultiplied(
    theme.primary[0],
    theme.primary[1],
    theme.primary[2],
    theme.primary[3],
  );
  let bg_color = egui::Color32::from_rgba_unmultiplied(
    theme.secondary[0],
    theme.secondary[1],
    theme.secondary[2],
    theme.secondary[3],
  );
  let text_color = ui.style().visuals.text_color();

  ui.vertical(|ui| {
    ui.painter()
      .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, bg_color);

    ui.centered_and_justified(|ui| {
      ui.set_max_width(800.0);

      ui.vertical(|ui| {
        ui.label(
          egui::RichText::new("FROM THE HACKERSPACE.")
            .color(primary_color)
            .font(font::cirka(48.0))
            .extra_letter_spacing(1.6),
        );

        ui.add_space(40.0);

        render_intro_animation(ui, world, text_color);
      });

      ui.vertical(|ui| {
        ui.add_space(100.0);

        ui.label(
          egui::RichText::new("WHY DO WE INNOVATE?")
            .color(primary_color)
            .font(font::cirka(48.0))
            .extra_letter_spacing(1.6),
        );

        ui.add_space(40.0);

        render_manifesto_animation(ui, world, text_color);

        ui.add_space(100.0);
      });
    });

    ui.add_space(100.0);
  });
}

fn render_intro_animation(
  ui: &mut egui::Ui,
  world: &mut World,
  text_color: egui::Color32,
) {
  let text_response = ui.allocate_response(
    egui::vec2(ui.available_width(), 200.0),
    egui::Sense::hover(),
  );

  let is_visible = ui.is_rect_visible(text_response.rect);

  let dt = world
    .get_resource::<DeltaTime>()
    .map(|t| t.delta())
    .unwrap_or(1.0 / 60.0);

  let (animated_text, still_animating) = {
    let Some(mut about) = world.get_resource_mut::<AboutResource>() else {
      return;
    };

    if is_visible && !about.intro_was_visible {
      about.intro_animation.reset();
    }

    about.intro_was_visible = is_visible;

    let animating = if is_visible {
      about.intro_animation.update(dt)
    } else {
      false
    };

    (about.intro_animation.visible_text(), animating)
  };

  if still_animating
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_hacker_active();
  }

  ui.scope_builder(egui::UiBuilder::new().max_rect(text_response.rect), |ui| {
    ui.label(
      egui::RichText::new(animated_text)
        .size(18.0)
        .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
        .color(text_color),
    );
  });
}

fn render_manifesto_animation(
  ui: &mut egui::Ui,
  world: &mut World,
  text_color: egui::Color32,
) {
  let text_response = ui.allocate_response(
    egui::vec2(ui.available_width(), 700.0),
    egui::Sense::hover(),
  );

  let is_visible = ui.is_rect_visible(text_response.rect);

  let dt = world
    .get_resource::<DeltaTime>()
    .map(|t| t.delta())
    .unwrap_or(1.0 / 60.0);

  let (animated_text, still_animating) = {
    let Some(mut about) = world.get_resource_mut::<AboutResource>() else {
      return;
    };

    if is_visible && !about.manifesto_was_visible {
      about.manifesto_animation.reset();
    }

    about.manifesto_was_visible = is_visible;

    let animating = if is_visible {
      about.manifesto_animation.update(dt)
    } else {
      false
    };

    (about.manifesto_animation.visible_text(), animating)
  };

  if still_animating
    && let Some(mut anim) = world.get_resource_mut::<ContinuousAnimations>()
  {
    anim.set_hacker_active();
  }

  ui.scope_builder(egui::UiBuilder::new().max_rect(text_response.rect), |ui| {
    ui.label(
      egui::RichText::new(animated_text)
        .size(18.0)
        .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
        .color(text_color),
    );
  });
}

fn render_footer(ui: &mut egui::Ui, world: &World) {
  let theme = get_theme(world);
  let primary_color = egui::Color32::from_rgba_unmultiplied(
    theme.primary[0],
    theme.primary[1],
    theme.primary[2],
    theme.primary[3],
  );
  let secondary_color = egui::Color32::from_rgba_unmultiplied(
    theme.secondary[0],
    theme.secondary[1],
    theme.secondary[2],
    theme.secondary[3],
  );

  let footer_rect =
    ui.allocate_space(egui::vec2(ui.available_width(), 600.0)).1;

  ui.painter().rect_filled(
    footer_rect,
    egui::CornerRadius::ZERO,
    primary_color,
  );

  ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
    ui.vertical_centered(|ui| {
      ui.add_space(100.0);

      let mut layout_job = egui::text::LayoutJob::default();

      layout_job.append(
        "reach",
        0.0,
        egui::TextFormat {
          font_id: font::aeonik(140.0),
          color: secondary_color,
          extra_letter_spacing: -4.0,
          ..Default::default()
        },
      );

      layout_job.append(
        "out",
        0.0,
        egui::TextFormat {
          font_id: font::cirka(150.0),
          color: secondary_color,
          extra_letter_spacing: -4.0,
          ..Default::default()
        },
      );

      ui.label(layout_job);
      ui.add_space(20.0);
      ui.spacing_mut().button_padding = egui::vec2(40.0, 10.0);

      let email_button = egui::Button::new(
        egui::RichText::new("THE [at] COMPiLORDS [dot] HOUSE")
          .size(14.0)
          .family(egui::FontFamily::Name(font::SUISSE_INTL.into()))
          .color(secondary_color),
      )
      .stroke(egui::Stroke::new(1.0_f32, secondary_color))
      .fill(egui::Color32::TRANSPARENT)
      .corner_radius(0.0);

      let button_response = ui.add(email_button);

      if button_response.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
      }
    });

    render_footer_bottom(ui, footer_rect, secondary_color);
  });
}

fn render_footer_bottom(
  ui: &mut egui::Ui,
  footer_rect: egui::Rect,
  text_color: egui::Color32,
) {
  let bottom_y = footer_rect.bottom() - 80.0;
  let bottom_rect = egui::Rect::from_min_size(
    egui::pos2(footer_rect.left(), bottom_y),
    egui::vec2(footer_rect.width(), 80.0),
  );

  ui.scope_builder(egui::UiBuilder::new().max_rect(bottom_rect), |ui| {
    ui.columns_const(|[lhs, mhs, rhs]| {
      lhs.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add_space(40.0);

        ui.vertical(|ui| {
          ui.label(
            egui::RichText::new("COMPiLORDS 2024")
              .size(10.0)
              .family(egui::FontFamily::Name(font::AEONIK.into()))
              .color(text_color),
          );

          ui.label(
            egui::RichText::new("ALL RiGHTS RESERVED.")
              .size(10.0)
              .family(egui::FontFamily::Name(font::AEONIK.into()))
              .color(text_color),
          );
        });
      });

      mhs.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.label(
          egui::RichText::new("CODELORD iS A COMPiLORDS SOFTWARE.")
            .size(12.0)
            .family(egui::FontFamily::Name(font::AEONIK.into()))
            .color(text_color),
        );

        ui.label(
          egui::RichText::new("BE AHEAD, JOiN THE DEVOLUTiON.")
            .size(12.0)
            .family(egui::FontFamily::Name(font::AEONIK.into()))
            .color(text_color),
        );
      });

      rhs.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        ui.add_space(40.0);

        ui.horizontal(|ui| {
          for (social, link) in [
            ("X", "https://x.com/invisageable"),
            ("DC", "https://discord.gg/JaNc4Nk5xw"),
            ("Gh", "https://github.com/sponsors/invisageable"),
          ]
          .iter()
          .rev()
          {
            let button = egui::Button::new(
              egui::RichText::new(*social)
                .size(14.0)
                .family(egui::FontFamily::Name(font::AEONIK.into()))
                .color(text_color),
            )
            .stroke(egui::Stroke::new(1.0_f32, text_color))
            .fill(egui::Color32::TRANSPARENT)
            .corner_radius(0.0)
            .min_size(egui::vec2(50.0, 40.0));

            let button_response = ui.add(button);

            if button_response.hovered() {
              ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            }

            if button_response.clicked() {
              ui.ctx().open_url(egui::OpenUrl::new_tab(link));
            }

            ui.add_space(10.0);
          }
        });
      });
    });
  });
}
