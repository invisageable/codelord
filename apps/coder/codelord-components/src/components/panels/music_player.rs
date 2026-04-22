//! Music player panel component.
//!
//! Provides a compact music player UI with:
//! - Play/pause, previous/next controls
//! - Volume control with mute toggle
//! - Repeat mode toggle
//! - Waveform visualization
//! - Time display (current / total)
//! - Playlist toggle

use crate::assets::icon::icon_to_image;
use crate::components::structure::divider::{self, LabelAlign};
use crate::components::waveform;

use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::audio::resources::{
  AudioDispatcher, MusicPlayerState, Playlist,
};
use codelord_core::ecs::world::World;
use codelord_core::icon::components::{Icon, Player};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// Accent color for active toggles (lime green).
const ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(204, 253, 62);

/// Default collapsed height for the player.
const PLAYER_HEIGHT: f32 = 40.0;

/// Expanded height when playlist is visible.
const PLAYLIST_HEIGHT: f32 = 200.0;

/// Show the music player panel with optional playlist.
pub fn show(ui: &mut egui::Ui, world: &mut World) {
  let current_time = world
    .get_resource::<DeltaTime>()
    .map(|t| t.elapsed())
    .unwrap_or(0.0);

  // Get visibility and playlist state, update animation.
  let (_playlist_visible, animated_height, is_animating) = {
    if let Some(mut state) = world.get_resource_mut::<MusicPlayerState>() {
      let playlist_visible = state.playlist_visible;

      // Only set target based on playlist if player is visible.
      // toggle_visibility handles show/hide targets.
      if state.visible {
        let target = if playlist_visible {
          PLAYER_HEIGHT + PLAYLIST_HEIGHT
        } else {
          PLAYER_HEIGHT
        };
        state.height_animation.set_target(target, current_time);
      }

      let height = state.height_animation.update(current_time);
      let animating = state.height_animation.is_animating();

      (playlist_visible, height, animating)
    } else {
      (false, PLAYER_HEIGHT, false)
    }
  };

  // Track animation state for the generic animation system.
  if is_animating
    && let Some(mut continuous) =
      world.get_resource_mut::<ContinuousAnimations>()
  {
    continuous.set_music_player_active();
  }

  // Use explicit vertical stacking.
  let available = ui.available_rect_before_wrap();

  // Player controls region (fixed 40px at top).
  let player_rect = egui::Rect::from_min_size(
    available.min,
    egui::vec2(available.width(), PLAYER_HEIGHT),
  );

  // Playlist region (below player).
  let playlist_height = animated_height - PLAYER_HEIGHT;
  let playlist_rect = egui::Rect::from_min_size(
    egui::pos2(available.min.x, available.min.y + PLAYER_HEIGHT),
    egui::vec2(available.width(), playlist_height.max(0.0)),
  );

  // Render player controls in their region.
  let mut player_ui = ui.new_child(
    egui::UiBuilder::new()
      .max_rect(player_rect)
      .layout(egui::Layout::left_to_right(egui::Align::Center)),
  );

  player_ui.columns_const(|[lhs, mhs, rhs]| {
    // Left column: playback controls.
    lhs.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      show_playback_controls(ui, world);
      show_time_display(ui, world);
    });

    // Middle column: waveform visualization.
    mhs.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      show_waveform(ui, world);
    });

    // Right column: track info and toggles.
    rhs.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
      ui.add_space(8.0);
      show_toggles(ui, world);
      show_track_info(ui, world);
    });
  });

  // Render playlist in its region (below player).
  if playlist_height > 1.0 {
    let mut playlist_ui = ui.new_child(
      egui::UiBuilder::new()
        .max_rect(playlist_rect)
        .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );

    show_playlist(&mut playlist_ui, world);
  }
}

/// Show play/pause, prev, next, volume, and repeat controls.
fn show_playback_controls(ui: &mut egui::Ui, world: &mut World) {
  let audio = world
    .get_resource::<AudioDispatcher>()
    .copied()
    .unwrap_or_default();

  // Extract values first, then drop the borrow.
  let (is_playing, is_muted, is_repeat) = {
    let state = world.get_resource::<MusicPlayerState>();
    (
      state.map(|s| s.is_playing).unwrap_or(false),
      state.map(|s| s.is_muted).unwrap_or(false),
      state.map(|s| s.is_repeat).unwrap_or(false),
    )
  };

  // Play/Pause button.
  let play_icon = if is_playing {
    Icon::Player(Player::Pause)
  } else {
    Icon::Player(Player::Play)
  };

  let play_btn = ui.add(
    egui::Button::image(
      icon_to_image(&play_icon)
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(13.0))
        .tint(egui::Color32::WHITE),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if play_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  if play_btn.clicked() {
    log::debug!("event:click:play-pause:music-player");

    // Snapshot the playlist so the mut borrow of MusicPlayerState
    // doesn't collide with the Playlist read.
    let playlist = world
      .get_resource::<Playlist>()
      .cloned()
      .unwrap_or_default();

    if let Some(mut state) = world.get_resource_mut::<MusicPlayerState>() {
      state.toggle(&audio, &playlist);
    }
  }

  // Volume button.
  let volume_icon = if is_muted {
    Icon::Player(Player::Muted)
  } else {
    Icon::Player(Player::Volume)
  };

  let volume_btn = ui.add(
    egui::Button::image(
      icon_to_image(&volume_icon)
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(18.0))
        .tint(egui::Color32::WHITE),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if volume_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  if volume_btn.clicked()
    && let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
  {
    state.toggle_mute();
    audio.music_set_volume(state.volume);
  }

  // Previous button (flipped next icon).
  let prev_btn = ui.add(
    egui::Button::image(
      icon_to_image(&Icon::Player(Player::Next))
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(12.0))
        .uv(egui::Rect::from_min_max(
          egui::pos2(1.0, 0.0),
          egui::pos2(0.0, 1.0),
        ))
        .tint(egui::Color32::WHITE),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if prev_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  // Next button.
  let next_btn = ui.add(
    egui::Button::image(
      icon_to_image(&Icon::Player(Player::Next))
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(12.0))
        .tint(egui::Color32::WHITE),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if next_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  // Repeat button.
  let replay_tint = if is_repeat {
    ACCENT_COLOR
  } else {
    egui::Color32::WHITE
  };

  let replay_btn = ui.add(
    egui::Button::image(
      icon_to_image(&Icon::Player(Player::Replay))
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(18.0))
        .tint(replay_tint),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if replay_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  if replay_btn.clicked()
    && let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
  {
    state.is_repeat = !state.is_repeat;

    audio.music_set_repeat(state.is_repeat);
  }
}

/// Show current time and total duration.
fn show_time_display(ui: &mut egui::Ui, world: &mut World) {
  let audio = world
    .get_resource::<AudioDispatcher>()
    .copied()
    .unwrap_or_default();

  // Update snapshot from audio system.
  if let Some(snapshot) = audio.music_snapshot()
    && let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
  {
    state.update_from_snapshot(&snapshot);

    state.snapshot = Some(snapshot);
  }

  let state = world.get_resource::<MusicPlayerState>();

  let (current_time, total_time) = if let Some(state) = state {
    if let Some(ref snapshot) = state.snapshot {
      let position = snapshot.position();
      let duration = snapshot.duration().unwrap_or(std::time::Duration::ZERO);

      (format_time(position), format_time(duration))
    } else {
      ("00:00".to_string(), "00:00".to_string())
    }
  } else {
    ("00:00".to_string(), "00:00".to_string())
  };

  ui.horizontal(|ui| {
    ui.label(
      egui::RichText::new(&current_time)
        .color(egui::Color32::WHITE)
        .size(8.0),
    );
    ui.add_space(-8.0);
    ui.label(
      egui::RichText::new(" - ")
        .color(egui::Color32::WHITE)
        .size(8.0),
    );
    ui.add_space(-8.0);
    ui.label(
      egui::RichText::new(&total_time)
        .color(egui::Color32::WHITE)
        .size(8.0),
    );
  });
}

/// Show the waveform visualization.
fn show_waveform(ui: &mut egui::Ui, world: &mut World) {
  let audio = world
    .get_resource::<AudioDispatcher>()
    .copied()
    .unwrap_or_default();

  // Acquire visualizer if needed (mutable access first).
  if let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
    && state.waveform.is_none()
    && state.is_playing
  {
    state.waveform = audio.music_visualizer();
  }

  // Now read state for display.
  let (is_playing, samples) = {
    if let Some(state) = world.get_resource::<MusicPlayerState>() {
      let samples = state
        .waveform
        .as_ref()
        .map(|v| v.get_normalized_samples().to_vec());

      (state.is_playing, samples)
    } else {
      (false, None)
    }
  };

  if let Some(samples) = samples {
    waveform::show(ui, &samples);
    // Track waveform animation through the generic animation system.
    if is_playing
      && let Some(mut continuous) =
        world.get_resource_mut::<ContinuousAnimations>()
    {
      continuous.set_music_player_active();
    }
  } else {
    waveform::show(ui, &[]);
  }
}

/// Show playlist and caption toggle buttons.
fn show_toggles(ui: &mut egui::Ui, world: &mut World) {
  let (playlist_visible, caption_visible) = {
    let state = world.get_resource::<MusicPlayerState>();
    (
      state.map(|s| s.playlist_visible).unwrap_or(false),
      state.map(|s| s.caption_visible).unwrap_or(false),
    )
  };

  // Playlist toggle.
  let playlist_tint = if playlist_visible {
    ACCENT_COLOR
  } else {
    egui::Color32::WHITE
  };

  let playlist_btn = ui.add(
    egui::Button::image(
      icon_to_image(&Icon::Player(Player::Playlist))
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(13.0))
        .tint(playlist_tint),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if playlist_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  if playlist_btn.clicked()
    && let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
  {
    state.playlist_visible = !state.playlist_visible;
  }

  // Caption toggle.
  let caption_tint = if caption_visible {
    ACCENT_COLOR
  } else {
    egui::Color32::WHITE
  };

  let caption_btn = ui.add(
    egui::Button::image(
      icon_to_image(&Icon::Quote)
        .fit_to_original_size(2.0)
        .max_size(egui::Vec2::splat(13.0))
        .tint(caption_tint),
    )
    .fill(egui::Color32::TRANSPARENT)
    .frame(false),
  );

  if caption_btn.hovered() {
    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
  }

  if caption_btn.clicked()
    && let Some(mut state) = world.get_resource_mut::<MusicPlayerState>()
  {
    state.caption_visible = !state.caption_visible;
  }
}

/// Show current track information.
fn show_track_info(ui: &mut egui::Ui, world: &mut World) {
  let state = world.get_resource::<MusicPlayerState>();

  let track_name = if let Some(state) = state {
    if let Some(ref snapshot) = state.snapshot {
      snapshot.track_name.clone().unwrap_or_default()
    } else {
      String::new()
    }
  } else {
    String::new()
  };

  if !track_name.is_empty() {
    ui.label(
      egui::RichText::new(&track_name)
        .color(egui::Color32::WHITE)
        .size(10.0),
    );
  }
}

/// Format a duration as MM:SS or HH:MM:SS.
fn format_time(duration: std::time::Duration) -> String {
  let total_secs = duration.as_secs();
  let hours = total_secs / 3600;
  let mins = (total_secs % 3600) / 60;
  let secs = total_secs % 60;

  if hours > 0 {
    format!("{hours:02}:{mins:02}:{secs:02}")
  } else {
    format!("{mins:02}:{secs:02}")
  }
}

/// Audio file extensions we accept.
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "aac", "m4a"];

/// Show the playlist panel.
fn show_playlist(ui: &mut egui::Ui, world: &mut World) {
  use codelord_audio::source::FileSource;
  use codelord_core::audio::resources::PlaylistEntry;

  let entries = world
    .get_resource::<Playlist>()
    .map(|p| p.entries.clone())
    .unwrap_or_default();

  // Handle drag and drop.
  let dropped_files: Vec<_> = ui
    .ctx()
    .input(|i| i.raw.dropped_files.clone())
    .into_iter()
    .filter_map(|f| f.path)
    .filter(|p| {
      p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
        AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
      })
    })
    .collect();

  if !dropped_files.is_empty()
    && let Some(mut playlist) = world.get_resource_mut::<Playlist>()
  {
    for path in dropped_files {
      // Extract metadata using symphonia via FileSource.
      let (title, artist, album, time) =
        if let Ok(source) = FileSource::new(path.clone()) {
          let metadata = source.metadata();

          let title = metadata.title.clone().unwrap_or_else(|| {
            path
              .file_stem()
              .and_then(|s| s.to_str())
              .unwrap_or("Unknown")
              .to_string()
          });

          let artist = metadata.artist.clone().unwrap_or_default();
          let album = metadata.album.clone().unwrap_or_default();

          let time = metadata
            .duration
            .map(format_time)
            .unwrap_or_else(|| "--:--".to_string());

          (title, artist, album, time)
        } else {
          let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

          (title, String::new(), String::new(), "--:--".to_string())
        };

      playlist.add(PlaylistEntry {
        title,
        artist,
        album,
        time,
        genre: String::new(),
        plays: 0,
        path,
      });
    }
  }

  // Divider with label.
  divider::show_with_label(ui, "PLAYLiST", LabelAlign::Center);
  ui.add_space(8.0);

  // Empty state.
  if entries.is_empty() {
    ui.centered_and_justified(|ui| {
      ui.label(
        egui::RichText::new("Drag and drop audio files here")
          .color(egui::Color32::from_gray(100))
          .size(12.0),
      );
    });

    return;
  }

  // Store paths for click handling.
  let paths = entries.iter().map(|e| e.path.clone()).collect::<Vec<_>>();
  let selected_idx = std::cell::Cell::new(None);

  // Disable row stroke for cleaner look.
  ui.style_mut().visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

  TableBuilder::new(ui)
    .id_salt("music-playlist")
    .striped(true)
    .sense(egui::Sense::click())
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
    .min_scrolled_height(0.0)
    .max_scroll_height(f32::INFINITY)
    .vscroll(true)
    .column(Column::remainder().resizable(true)) // Title + Artist
    .column(Column::initial(60.0).resizable(true)) // Time
    .column(Column::initial(150.0).resizable(true)) // Album
    .column(Column::initial(100.0).resizable(true)) // Genre
    .column(Column::initial(80.0).resizable(false)) // Plays
    .header(20.0, |mut header| {
      header.col(|ui| {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Title").color(ACCENT_COLOR).size(10.0));
      });
      header.col(|ui| {
        ui.label(egui::RichText::new("Time").color(ACCENT_COLOR).size(10.0));
      });
      header.col(|ui| {
        ui.label(egui::RichText::new("Album").color(ACCENT_COLOR).size(10.0));
      });
      header.col(|ui| {
        ui.label(egui::RichText::new("Genre").color(ACCENT_COLOR).size(10.0));
      });
      header.col(|ui| {
        ui.label(egui::RichText::new("Plays").color(ACCENT_COLOR).size(10.0));
      });
    })
    .body(|body| {
      body.rows(40.0, entries.len(), |mut row| {
        let idx = row.index();
        let entry = &entries[idx];

        // Title + Artist
        row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;
          ui.add_space(8.0);
          ui.vertical(|ui| {
            ui.label(
              egui::RichText::new(&entry.title)
                .color(egui::Color32::WHITE)
                .size(10.0),
            );
            ui.label(
              egui::RichText::new(&entry.artist)
                .color(egui::Color32::from_gray(64))
                .size(10.0),
            );
          });
        });
        // Time
        row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;
          ui.label(
            egui::RichText::new(&entry.time)
              .color(egui::Color32::WHITE)
              .size(10.0),
          );
        });
        // Album
        row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;
          ui.label(
            egui::RichText::new(&entry.album)
              .color(egui::Color32::WHITE)
              .size(10.0),
          );
        });
        // Genre
        row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;
          ui.label(
            egui::RichText::new(&entry.genre)
              .color(egui::Color32::WHITE)
              .size(10.0),
          );
        });
        // Plays
        row.col(|ui| {
          ui.style_mut().interaction.selectable_labels = false;
          ui.label(
            egui::RichText::new(format!("{}", entry.plays))
              .color(egui::Color32::WHITE)
              .size(10.0),
          );
        });

        // Handle row double-click.
        let response = row.response();
        if response.double_clicked() {
          selected_idx.set(Some(idx));
        }
      });
    });

  // Handle click outside table to avoid borrow conflicts.
  if let Some(idx) = selected_idx.get()
    && let Some(path) = paths.get(idx)
  {
    log::debug!("Playing track: {}", path.display());

    let audio = world
      .get_resource::<AudioDispatcher>()
      .copied()
      .unwrap_or_default();

    audio.music_play(path.clone());

    if let Some(mut playlist) = world.get_resource_mut::<Playlist>() {
      playlist.current_index = Some(idx);
    }
    if let Some(mut state) = world.get_resource_mut::<MusicPlayerState>() {
      state.is_playing = true;
    }
  }
}
