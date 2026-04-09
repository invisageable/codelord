use crate::components::atoms::stripe_button;
use crate::components::renderers::markdown;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::codeshow::{
  CodeshowState, NavigateSlide, PendingPresentationDirectory,
  PendingPresentationFile, SlideDirection,
};
use codelord_core::dialog;
use codelord_core::ecs::world::World;
use codelord_core::runtime::RuntimeHandle;

use eazy::easing::Curve;
use eazy::easing::polynomial::cubic::InOutCubic;

use eframe::egui;

pub fn show(ui: &mut egui::Ui, world: &mut World) {
  handle_keyboard(ui, world);

  // Get state
  let (
    is_loaded,
    current,
    total,
    slcodelord_content,
    transition_progress,
    is_animating,
  ) = {
    let state = world.get_resource::<CodeshowState>();

    match state {
      Some(s) => (
        s.is_loaded(),
        s.current,
        s.total,
        s.current_slide().map(|s| s.to_string()),
        s.transition_progress,
        s.is_animating(),
      ),
      None => (false, 0, 0, None, 1.0, false),
    }
  };

  // Mark animation as active if transitioning
  if is_animating
    && let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>()
  {
    cont.set_presenter_active();
  }

  if !is_loaded {
    show_empty_state(ui, world);
    return;
  }

  let visuals = ui.style().visuals.clone();

  // Left sidebar with thumbnails
  egui::Panel::left("presenter_slides")
    .default_size(160.0)
    .min_size(120.0)
    .max_size(240.0)
    .resizable(true)
    .frame(egui::Frame::NONE.fill(visuals.faint_bg_color))
    .show_inside(ui, |ui| {
      // Bottom counter (like explorer file count)
      egui::Panel::top("presenter_slcodelord_counter")
        .exact_size(24.0)
        .frame(
          egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill),
        )
        .show_inside(ui, |ui| {
          ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
              ui.add_space(8.0);
              ui.label(
                egui::RichText::new(format!("{} / {total}", current + 1))
                  .size(10.0)
                  .color(ui.visuals().weak_text_color()),
              );
            },
          );
        });

      egui::CentralPanel::default()
        .frame(
          egui::Frame::NONE.fill(ui.ctx().global_style().visuals.window_fill),
        )
        .show_inside(ui, |ui| show_slcodelord_thumbnails(ui, world));
    });

  // Top bar: + button and slide counter
  egui::Panel::top("presenter_toolbar")
    .exact_size(24.0)
    .frame(
      egui::Frame::NONE
        .fill(visuals.window_fill)
        .inner_margin(egui::Margin::symmetric(8, 0)),
    )
    .show_inside(ui, |ui| codeshow::presenter::show_toolbar(ui, world));

  // Bottom panel: presenter notes (editable)
  let (current_idx, mut note_text) = {
    let state = world.get_resource::<CodeshowState>();

    match state {
      Some(s) => (s.current, s.current_notes().unwrap_or("").to_string()),
      None => (0, String::new()),
    }
  };

  egui::Panel::bottom("presenter_notes")
    .min_size(80.0)
    .max_size(200.0)
    .resizable(true)
    .frame(egui::Frame::NONE.fill(visuals.faint_bg_color))
    .show_inside(ui, |ui| {
      ui.add_space(8.0);
      ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(
          egui::RichText::new("Notes")
            .size(11.0)
            .color(ui.visuals().weak_text_color()),
        );
      });
      ui.add_space(4.0);
      ui.separator();

      egui::ScrollArea::vertical()
        .id_salt("presenter_notes_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
          ui.add_space(8.0);
          let response = ui.add_sized(
            [ui.available_width() - 24.0, ui.available_height() - 16.0],
            egui::TextEdit::multiline(&mut note_text)
              .font(egui::TextStyle::Body)
              .hint_text("Add notes for this slide...")
              .frame(egui::Frame::NONE),
          );

          // Save notes when changed
          if response.changed()
            && let Some(mut state) = world.get_resource_mut::<CodeshowState>()
          {
            state.set_note(current_idx, note_text.clone());
          }

          ui.add_space(8.0);
        });
    });

  // Main content: slide
  egui::CentralPanel::default()
    .frame(egui::Frame::NONE.fill(visuals.extreme_bg_color))
    .show_inside(ui, |ui| {
      if let Some(content) = slcodelord_content {
        // Apply fade animation using eazy
        let eased = InOutCubic.y(transition_progress);
        ui.set_opacity(eased);

        ui.add_space(24.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
          ui.add_space(16.0);
          markdown::render(ui, &content);
          ui.add_space(16.0);
        });
      }
    });
}

fn show_empty_state(ui: &mut egui::Ui, world: &mut World) {
  // Check if dialog is already pending
  let file_pending = world.get_resource::<PendingPresentationFile>().is_some();
  let dir_pending = world
    .get_resource::<PendingPresentationDirectory>()
    .is_some();

  // Get runtime handle for async dialogs
  let runtime = world.get_resource::<RuntimeHandle>().cloned();

  ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
    ui.add_space(100.0);

    let button_width = 140.0;
    let spacing = 8.0;
    let total_width = button_width * 2.0 + spacing;
    let available = ui.available_width();
    let left_pad = (available - total_width) / 2.0;

    ui.horizontal(|ui| {
      ui.add_space(left_pad);

      // Open single file (non-blocking)
      let has_runtime = runtime.is_some();
      ui.add_enabled_ui(!file_pending && !dir_pending && has_runtime, |ui| {
        let btn_open_file = stripe_button::show(
          ui,
          world,
          "OPEN FILE",
          egui::vec2(button_width, 32.0),
        );

        if btn_open_file.hovered() {
          ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if btn_open_file.clicked()
          && let Some(ref rt) = runtime
        {
          let rx = dialog::pick_file(rt, &[("Markdown", &["md"])]);
          world.insert_resource(PendingPresentationFile(rx));
        }
      });

      ui.add_space(spacing);

      // Open directory (non-blocking)
      ui.add_enabled_ui(!file_pending && !dir_pending && has_runtime, |ui| {
        let btn_open_folder = stripe_button::show(
          ui,
          world,
          "OPEN FOLDER",
          egui::vec2(button_width, 32.0),
        );

        if btn_open_folder.hovered() {
          ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if btn_open_folder.clicked()
          && let Some(ref rt) = runtime
        {
          let rx = dialog::pick_folder(rt);
          world.insert_resource(PendingPresentationDirectory(rx));
        }
      });
    });

    ui.add_space(20.0);

    ui.label(
      egui::RichText::new("Single file: Use --- to separate slides")
        .size(12.0)
        .color(ui.visuals().weak_text_color()),
    );

    ui.label(
      egui::RichText::new(
        "Multi-file: One .md file per slide (sorted alphabetically)",
      )
      .size(12.0)
      .color(ui.visuals().weak_text_color()),
    );
  });
}

/// Show slide thumbnails in sidebar.
fn show_slcodelord_thumbnails(ui: &mut egui::Ui, world: &mut World) {
  const THUMB_WIDTH: f32 = 120.0;
  const THUMB_HEIGHT: f32 = 80.0;
  const THUMB_MARGIN: f32 = 16.0;
  const CORNER_RADIUS: f32 = 0.0;

  // Extract data from resource
  let (current, slides) = {
    let state = world.get_resource::<CodeshowState>();

    match state {
      Some(s) => (s.current, s.slides.clone()),
      None => return,
    }
  };

  egui::ScrollArea::vertical()
    .id_salt("presenter_thumbnails")
    .auto_shrink([false; 2])
    .show(ui, |ui| {
      ui.vertical_centered(|ui| {
        ui.add_space(THUMB_MARGIN);

        for (idx, content) in slides.iter().enumerate() {
          let is_current = idx == current;

          // Horizontal layout: number on left, thumbnail on right
          let response = ui.horizontal(|ui| {
            // Slide number (left side, vertically centered)
            ui.add_space(4.0);
            ui.label(
              egui::RichText::new(format!("{}", idx + 1))
                .size(12.0)
                .color(if is_current {
                  egui::Color32::from_rgb(90, 160, 255)
                } else {
                  egui::Color32::from_gray(120)
                }),
            );
            ui.add_space(8.0);

            // Thumbnail with fixed size and rounded corners
            let thumb_size = egui::vec2(THUMB_WIDTH, THUMB_HEIGHT);
            let (rect, _response) =
              ui.allocate_exact_size(thumb_size, egui::Sense::hover());

            // Draw background and border
            let rounding = egui::CornerRadius::same(CORNER_RADIUS as u8);
            ui.painter().rect(
              rect,
              rounding,
              egui::Color32::from_gray(15),
              if is_current {
                egui::Stroke::new(3.0, egui::Color32::from_rgb(60, 130, 240))
              } else {
                egui::Stroke::new(1.0, egui::Color32::from_gray(50))
              },
              egui::StrokeKind::Outside,
            );

            // Render content inside clipped area (non-selectable)
            let content_rect = rect.shrink(4.0);
            let mut child_ui = ui.new_child(
              egui::UiBuilder::new()
                .max_rect(content_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            child_ui.style_mut().interaction.selectable_labels = false;

            // Use ScrollArea to clip content (disabled scrolling)
            egui::ScrollArea::neither()
              .id_salt(("thumb_clip", idx))
              .max_height(THUMB_HEIGHT - 8.0)
              .max_width(THUMB_WIDTH - 8.0)
              .show(&mut child_ui, |ui| markdown::render_mini(ui, content));
          });

          // Click to navigate
          if response.response.interact(egui::Sense::click()).clicked()
            && let Some(mut state) = world.get_resource_mut::<CodeshowState>()
          {
            state.goto(idx);
          }

          ui.add_space(THUMB_MARGIN);
        }
      });
    });
}

fn handle_keyboard(ui: &egui::Ui, world: &mut World) {
  ui.ctx().input(|i| {
    // Next slide: Right Arrow, Up Arrow, Space, l, Enter, Page Up (NORWII N76)
    if i.key_pressed(egui::Key::ArrowRight)
      || i.key_pressed(egui::Key::ArrowDown)
      || i.key_pressed(egui::Key::Space)
      || i.key_pressed(egui::Key::L)
      || i.key_pressed(egui::Key::Enter)
      || i.key_pressed(egui::Key::PageDown)
    {
      world.spawn(NavigateSlide {
        direction: SlideDirection::Next,
      });
    }

    // Previous slide: Left Arrow, Down Arrow, Backspace, h, Page Down (NORWII
    // N76)
    if i.key_pressed(egui::Key::ArrowLeft)
      || i.key_pressed(egui::Key::ArrowUp)
      || i.key_pressed(egui::Key::Backspace)
      || i.key_pressed(egui::Key::H)
      || i.key_pressed(egui::Key::PageUp)
    {
      world.spawn(NavigateSlide {
        direction: SlideDirection::Previous,
      });
    }

    // First slide: Home, g
    if i.key_pressed(egui::Key::Home) || i.key_pressed(egui::Key::G) {
      world.spawn(NavigateSlide {
        direction: SlideDirection::First,
      });
    }

    // Last slide: End
    if i.key_pressed(egui::Key::End) {
      world.spawn(NavigateSlide {
        direction: SlideDirection::Last,
      });
    }
  });
}

pub mod codeshow {
  pub mod presenter {
    use crate::components::atoms::icon_button;

    use codelord_core::ecs::world::World;
    use codelord_core::icon::components::Icon;

    use eframe::egui;

    pub fn show_toolbar(ui: &mut egui::Ui, _world: &mut World) {
      ui.horizontal(|ui| {
        ui.with_layout(
          egui::Layout::left_to_right(egui::Align::Center),
          |ui| {
            let tint = ui.visuals().weak_text_color();

            if icon_button::show(ui, &Icon::Add, tint) {
              log::debug!("add slide");
              // we'll need to find a new way to add slide to the related
              // markdown slide.
            }
          },
        );
        ui.with_layout(
          egui::Layout::right_to_left(egui::Align::Center),
          |_ui| {
            // todo
          },
        );
      });
    }
  }
}
