use codelord_components::assets;
use codelord_components::components::indicators::frame_history;
use codelord_components::components::layouts::base;
use codelord_components::components::renderers::svg;
use codelord_components::components::structure;
use codelord_components::components::{effects, organisms, overlays, panels};
use codelord_components::radius;
use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::{
  ActiveAnimations, CenterWindowAnimation, ContinuousAnimations, ShakeAnimation,
};
use codelord_core::audio::resources::{AudioDispatcher, MusicPlayerState};
use codelord_core::ecs::schedule::Schedule;
use codelord_core::ecs::world::World;
use codelord_core::events::{
  CenterWindowRequest, ClearSessionRequest, CompileRequest,
  NavigateNextTabRequest, NavigatePrevTabRequest,
  PositionWindowLeftHalfRequest, PositionWindowRightHalfRequest,
  SaveFileRequest, ShakeWindowRequest, ToggleBlameRequest, ToggleSearchRequest,
};
use codelord_core::filescope::resources::{FilescopeMode, FilescopeState};
use codelord_core::loading::{GlobalLoading, LoadingTask};
use codelord_core::magic_zoom::{MagicZoomCommand, MagicZoomState};
use codelord_core::page::components::Page;
use codelord_core::page::resources::PageResource;
use codelord_core::playground::{
  PLAYGROUND_PREVIEW_URL, PlaygroundOutput, PlaygroundWebviewState,
};
use codelord_core::previews::sqlite::SqliteQuery;
use codelord_core::previews::{
  DEFAULT_PREVIEW_URL, HtmlPreviewState, PdfConnection, PdfPreviewState,
  SqliteConnection, SqlitePreviewState,
};
use codelord_core::search::SearchState;
use codelord_core::tabbar::components::EditorTab;
use codelord_core::theme::resources::{ThemeAction, ThemeCommand};
use codelord_core::toast::resources::{DismissToastCommand, ToastCommand};
use codelord_core::ui::component::Active;
use codelord_core::voice::components::VoiceState;
use codelord_core::voice::resources::{
  ModelStatus, VisualizerStatus, VoiceActionEvent, VoiceModelState,
  VoiceResource, VoiceToggleCommand,
};
use codelord_core::{
  about, animation, audio, codeshow, color, drag_and_drop, ecs, filescope, git,
  instruction, keyboard, loading, magic_zoom, navigation, page, panel,
  playground, popup, previews, remote, runtime, search, settings, statusbar,
  symbol, tabbar, terminal, text_editor, theme, titlebar, toast, voice, xmb,
};
use codelord_protocol::compilation::CompilationEvent;
use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::model::VoiceAction;
use codelord_sdk::Sdk;
use codelord_sdk::voice::DownloadResult;
use codelord_voice::{VoiceManager, transcriber};

use eframe::egui;
use flume::Receiver;
use raw_window_handle::HasWindowHandle;
use swisskit::renderer::html::HtmlRenderer;

use std::sync::Arc;

/// Production-ready IDE Application
pub struct Coder {
  /// ECS World - source of truth for all state
  world: World,
  /// System schedule - runs every frame
  schedule: Schedule,
  /// Tokio runtime for async operations (voice, server).
  /// Kept for ownership to keep runtime alive.
  #[allow(dead_code)]
  runtime: tokio::runtime::Runtime,
  /// SDK for server communication (voice, preview)
  sdk: Arc<Sdk>,
  /// Voice manager - controls recording, transcription, dispatching
  voice_manager: Option<VoiceManager>,
  /// Channel to receive voice actions from dispatcher
  voice_action_rx: Receiver<VoiceAction>,
  /// Previous voice state (to detect transitions)
  prev_voice_state: VoiceState,
  /// Voice model download receiver (when download is in progress)
  voice_model_download_rx: Option<Receiver<DownloadResult>>,
  /// Previous visualizer status (to detect error transitions)
  prev_visualizer_status: VisualizerStatus,
  /// Shake animation for error feedback
  shake_animation: Option<ShakeAnimation>,
  center_animation: Option<CenterWindowAnimation>,
  /// HTML preview WebView. Stored outside ECS because `wry::WebView`
  /// is `!Send + !Sync`.
  html_preview_webview: HtmlRenderer,
  html_preview_handle_set: bool,
  /// Playground WebView for templating mode. Stored outside ECS for
  /// the same reason as `html_preview_webview`.
  playground_webview: HtmlRenderer,
  playground_handle_set: bool,
  /// Flag to clear session on next save (instead of saving)
  clear_session_on_save: bool,
  /// Channel to receive compilation events from server
  compilation_event_rx: Option<Receiver<CompilationEvent>>,
}

impl Coder {
  /// Create a new IDE application
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    assets::install_assets(&cc.egui_ctx);
    effects::wave::WaveCallback::init(cc);

    let mut world = World::new();
    let initial_theme = assets::theme::get_theme(&world);

    cc.egui_ctx
      .set_visuals(assets::theme::theme_to_visuals(initial_theme));

    theme::install(&mut world);
    page::install(&mut world);
    animation::install(&mut world);
    loading::install(&mut world);
    navigation::install(&mut world);
    symbol::install(&mut world);
    text_editor::install(&mut world);
    codelord_language::install_symbol_extractors(&mut world);
    codelord_language::install_token_extractors(&mut world);
    color::install(&mut world, codelord_language::color::extract);
    statusbar::install(&mut world);
    panel::install(&mut world);
    audio::install(&mut world);
    previews::install(&mut world);
    // SVG texture cache is non-Send (holds an egui TextureHandle), so it
    // lives in codelord-components — see `svg::install_non_send`.
    svg::install_non_send(&mut world);
    search::install(&mut world);
    popup::install(&mut world);
    tabbar::install(&mut world);
    drag_and_drop::install(&mut world);
    keyboard::install(&mut world);
    xmb::install(&mut world);
    magic_zoom::install(&mut world);
    about::install(&mut world);
    settings::install(&mut world);
    git::install(&mut world);
    instruction::install(&mut world);
    toast::install(&mut world);
    terminal::install(&mut world);
    voice::install(&mut world);
    filescope::install(&mut world);
    codeshow::install(&mut world);
    playground::install(&mut world);
    remote::install(&mut world);

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(2)
      .enable_all()
      .build()
      .expect("Failed to create Tokio runtime");

    runtime::install(&mut world, runtime.handle().clone());

    let sdk = Arc::new(Sdk::new(runtime.handle().clone()));
    let (voice_action_tx, voice_action_rx) = flume::unbounded::<VoiceAction>();
    let visualizer_state = codelord_voice::install_visualizer(&mut world);

    let voice_manager = VoiceManager::new(
      voice_action_tx,
      None,
      runtime.handle().clone(),
      sdk.clone(),
      visualizer_state,
    )
    .map_err(|err| {
      log::warn!("Voice manager initialization failed: {err}");

      err
    })
    .ok();

    voice_manager.as_ref().map(|vm| {
      world
        .get_resource_mut::<VoiceResource>()
        .map(|mut voice_res| voice_res.is_available = vm.is_available())
    });

    if let Some(mut model_state) = world.get_resource_mut::<VoiceModelState>() {
      if transcriber::model_exists() {
        model_state.set_ready();

        log::info!(
          "[Voice] Model found at: {}",
          transcriber::model_path().display()
        );
      } else {
        model_state.status = ModelStatus::Missing;

        log::info!("[Voice] Model not found, will prompt on first use");
      }
    }

    titlebar::spawn_default(&mut world);
    statusbar::spawn_default_icons(&mut world);
    settings::spawn_popup(&mut world);
    navigation::spawn_context_popup(&mut world);
    tabbar::spawn_context_popup(&mut world);
    previews::sqlite::spawn_export_popup(&mut world);

    let session_restored = Self::restore_session(cc, &mut world);

    if !session_restored {
      playground::spawn_default_tab(&mut world);
    }

    let mut schedule = Schedule::default();

    theme::register_systems(&mut schedule);
    page::register_systems(&mut schedule);
    magic_zoom::register_systems(&mut schedule);
    navigation::register_systems(&mut schedule);
    keyboard::register_systems(&mut schedule);
    text_editor::register_systems(&mut schedule);
    symbol::register_systems(&mut schedule);
    git::register_systems(&mut schedule);
    playground::register_systems(&mut schedule);
    statusbar::register_systems(&mut schedule);
    tabbar::register_systems(&mut schedule);
    panel::register_systems(&mut schedule);
    popup::register_systems(&mut schedule);
    xmb::register_systems(&mut schedule);
    terminal::register_systems(&mut schedule);
    toast::register_systems(&mut schedule);
    voice::register_systems(&mut schedule);
    filescope::register_systems(&mut schedule);
    previews::register_systems(&mut schedule);
    remote::register_systems(&mut schedule);

    let (compilation_tx, compilation_rx) = flume::unbounded();
    let sdk_clone = Arc::clone(&sdk);

    runtime.spawn(async move {
      match sdk_clone.connect_events().await {
        Ok(event_rx) => {
          log::info!("[Compilation] Connected to event stream");

          while let Ok(event) = event_rx.recv_async().await {
            if let ServerEvent::Compilation(compilation_event) = event {
              let _ = compilation_tx.send_async(compilation_event).await;
            }
          }

          log::warn!("[Compilation] Event stream closed");
        }
        Err(err) => {
          log::warn!("[Compilation] Failed to connect to events: {err}");
        }
      }
    });

    Self {
      world,
      schedule,
      runtime,
      sdk,
      voice_manager,
      voice_action_rx,
      prev_voice_state: VoiceState::Idle,
      voice_model_download_rx: None,
      prev_visualizer_status: VisualizerStatus::Idle,
      shake_animation: None,
      center_animation: None,
      html_preview_webview: HtmlRenderer::new(DEFAULT_PREVIEW_URL),
      html_preview_handle_set: false,
      playground_webview: HtmlRenderer::new(PLAYGROUND_PREVIEW_URL),
      playground_handle_set: false,
      clear_session_on_save: false,
      compilation_event_rx: Some(compilation_rx),
    }
  }
}

impl eframe::App for Coder {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
  }

  /// Called once when the app is shutting down. Stops the music and
  /// terminates the dedicated audio thread cleanly.
  fn on_exit(&mut self) {
    let audio = self
      .world
      .get_resource::<AudioDispatcher>()
      .copied()
      .unwrap_or_default();

    audio.shutdown();
  }

  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    // Wrap the app body in a transformed layer when zoomed. Overlays
    // render as top-level `Area`s outside this wrap, so they get their
    // own transform pass in `effects::magic_zoom::propagate`. `None`
    // keeps the identity case zero-cost.
    let transform = effects::magic_zoom::transform(&self.world);

    egui::CentralPanel::default()
      .frame(egui::Frame::NONE)
      .show_inside(ui, |ui| {
        let Some(transform) = transform else {
          self.show(ui);

          return;
        };

        let layer_id = effects::magic_zoom::layer_id();

        ui.ctx().set_transform_layer(layer_id, transform);

        ui.scope_builder(
          egui::UiBuilder::new()
            .layer_id(layer_id)
            .max_rect(ui.available_rect_before_wrap()),
          |ui| self.show(ui),
        );
      });
  }

  fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    frame_history::record_frame_time(ctx, frame);

    let delta = ctx.input(|i| i.stable_dt);

    if let Some(mut dt) = self.world.get_resource_mut::<DeltaTime>() {
      dt.update(delta);
    }

    while let Ok(voice_action) = self.voice_action_rx.try_recv() {
      log::info!("[Voice] Received action: {}", voice_action.action);

      self.world.write_message(VoiceActionEvent {
        action: voice_action.action,
        payload: voice_action.payload,
      });
    }

    if let Some(rx) = &self.voice_model_download_rx {
      while let Ok(result) = rx.try_recv() {
        match result {
          codelord_sdk::voice::DownloadResult::Progress(progress) => {
            if let Some(mut model_state) =
              self.world.get_resource_mut::<VoiceModelState>()
            {
              model_state.set_progress(progress.fraction);
            }
          }
          codelord_sdk::voice::DownloadResult::Complete(_path) => {
            log::info!("[Voice] Model download complete");

            if let Some(mut loading) =
              self.world.get_resource_mut::<GlobalLoading>()
            {
              loading.finish(LoadingTask::Network);
            }

            if let Some(ref mut vm) = self.voice_manager
              && vm.load_transcriber()
            {
              if let Some(mut voice) =
                self.world.get_resource_mut::<VoiceResource>()
              {
                voice.is_available = true;
              }
            }

            if let Some(mut model_state) =
              self.world.get_resource_mut::<VoiceModelState>()
            {
              model_state.set_ready();
            }

            self
              .world
              .write_message(ToastCommand::success("Voice model ready"));

            self.voice_model_download_rx = None;

            break;
          }
          codelord_sdk::voice::DownloadResult::Error(err) => {
            log::error!("[Voice] Model download failed: {err}");

            if let Some(mut loading) =
              self.world.get_resource_mut::<GlobalLoading>()
            {
              loading.finish(LoadingTask::Network);
            }

            if let Some(mut model_state) =
              self.world.get_resource_mut::<VoiceModelState>()
            {
              model_state.set_error(&err);
            }

            self.world.write_message(ToastCommand::error(format!(
              "Voice model download failed: {err}"
            )));

            self.voice_model_download_rx = None;

            break;
          }
        }
      }
    }

    // Collect events first to avoid borrow conflict.
    let compilation_events: Vec<_> = self
      .compilation_event_rx
      .as_mut()
      .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
      .unwrap_or_default();

    for event in compilation_events {
      playground::apply_compilation_event(&mut self.world, event);
    }

    // SQLite open stays here because it needs `runtime.block_on`. All
    // other SQLite lifecycle (query dispatch, result polling, close)
    // lives in `codelord_core::previews::sqlite`.
    {
      let need_open = self
        .world
        .get_resource::<SqlitePreviewState>()
        .filter(|s| s.enabled && s.needs_reload && !s.is_loading)
        .and_then(|s| s.current_file.clone())
        .filter(|_| {
          self
            .world
            .get_resource::<SqliteConnection>()
            .map(|c| !c.is_connected())
            .unwrap_or(true)
        });

      if let Some(file) = need_open {
        let path_str = file.to_string_lossy().to_string();

        log::info!("[SQLite] Opening database: {path_str}");

        if let Some(mut state) =
          self.world.get_resource_mut::<SqlitePreviewState>()
        {
          state.is_loading = true;
        }

        let runtime_handle = self.runtime.handle().clone();

        match self.runtime.block_on(codelord_sdk::sqlite::open_database(
          &path_str,
          &runtime_handle,
        )) {
          Ok((query_tx, result_rx)) => {
            let _ = query_tx.send(SqliteQuery::LoadTables);

            if let Some(mut connection) =
              self.world.get_resource_mut::<SqliteConnection>()
            {
              connection.set(query_tx, result_rx);
            }

            log::info!("[SQLite] Database opened, loading tables...");
          }
          Err(err) => {
            log::error!("[SQLite] Failed to open database: {err}");

            if let Some(mut state) =
              self.world.get_resource_mut::<SqlitePreviewState>()
            {
              state.data.error = Some(err);
              state.is_loading = false;
              state.needs_reload = false;
            }
          }
        }
      }
    }

    // PDF open stays here because it spawns a background rendering
    // thread tied to the app lifecycle. Everything else (tab-change
    // handling, loading state, animations) lives in ECS systems
    // under `codelord_core::previews::pdf`.
    {
      let on_editor_page = self
        .world
        .get_resource::<PageResource>()
        .map(|p| p.active_page == Page::Editor)
        .unwrap_or(false);

      let need_open = on_editor_page
        .then(|| {
          self
            .world
            .get_resource::<PdfPreviewState>()
            .filter(|s| s.enabled && s.is_loading)
            .and_then(|s| s.current_file.clone())
            .filter(|_| {
              self
                .world
                .get_resource::<PdfConnection>()
                .map(|c| !c.is_connected())
                .unwrap_or(true)
            })
        })
        .flatten();

      if let Some(file) = need_open {
        log::info!("[PDF] Opening file: {}", file.display());

        match codelord_sdk::pdf::open_pdf(&file) {
          Ok((query_tx, result_rx)) => {
            if let Some(mut conn) =
              self.world.get_resource_mut::<PdfConnection>()
            {
              conn.set(query_tx, result_rx);
            }

            log::info!("[PDF] File opened, worker started");
          }
          Err(err) => {
            log::error!("[PDF] Failed to open file: {err}");

            if let Some(mut state) =
              self.world.get_resource_mut::<PdfPreviewState>()
            {
              state.set_error(err);
            }
          }
        }
      }
    }

    let compile_requests: Vec<_> = self
      .world
      .query_filtered::<(bevy_ecs::entity::Entity, &CompileRequest), ()>()
      .iter(&self.world)
      .map(|(err, req)| {
        (err, req.source.clone(), req.target.clone(), req.stage)
      })
      .collect();

    for (entity, source, target, stage) in compile_requests {
      log::info!(
        "[Compilation] Triggering compilation for source ({} bytes, stage {stage:?})",
        source.len(),
      );

      if let Some(mut output) =
        self.world.get_resource_mut::<PlaygroundOutput>()
      {
        output.compilation.is_compiling = true;
        output.compilation.tokens = None;
        output.compilation.tree = None;
        output.compilation.sir = None;
        output.compilation.asm = None;
        output.compilation.ui = None;
      }

      self.sdk.compile(source, target, stage);
      self.world.despawn(entity);
    }

    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<ClearSessionRequest>>()
      .iter(&self.world)
      .next()
    {
      self.clear_session_on_save = true;
      self.world.despawn(entity);
      codelord_core::session::reset_to_fresh_state(&mut self.world);
      log::info!("[Session] Session cleared and state reset");
    }

    // Window-request handlers below all need `ctx.send_viewport_cmd`,
    // so they stay here rather than in a core system.
    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<CenterWindowRequest>>()
      .iter(&self.world)
      .next()
    {
      log::info!("[Window] Centering window on screen");

      self.world.despawn(entity);

      let animation_data = ctx.input(|i| {
        let current_pos = i.viewport().outer_rect.map(|r| r.min)?;
        let monitor_size = i.viewport().monitor_size?;
        let inner_rect = i.viewport().inner_rect?;
        let window_size = inner_rect.size();

        let center_x = (monitor_size.x - window_size.x) / 2.0;
        let center_y = (monitor_size.y - window_size.y) / 2.0;
        let center_pos = egui::pos2(center_x, center_y);

        Some((current_pos, center_pos, i.time))
      });

      if let Some((start_pos, end_pos, current_time)) = animation_data {
        self.center_animation = Some(CenterWindowAnimation::new(
          current_time,
          start_pos.x,
          start_pos.y,
          end_pos.x,
          end_pos.y,
        ));

        if let Some(mut active) = self.world.get_resource_mut::<ActiveAnimations>()
        {
          active.increment();
        }
      }
    }

    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<ShakeWindowRequest>>()
      .iter(&self.world)
      .next()
    {
      log::info!("[Window] Shaking window");

      self.world.despawn(entity);

      if let Some(pos) = ctx.input(|i| i.viewport().outer_rect.map(|r| r.min)) {
        let current_time = ctx.input(|i| i.time);

        self.shake_animation =
          Some(ShakeAnimation::new(current_time, pos.x, pos.y));

        if let Some(mut active) = self.world.get_resource_mut::<ActiveAnimations>()
        {
          active.increment();
        }
      }
    }

    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<PositionWindowLeftHalfRequest>>()
      .iter(&self.world)
      .next()
    {
      log::info!("[Window] Positioning window to left half");
      self.world.despawn(entity);

      if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
        let half_width = monitor_size.x / 2.0;

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
          0.0, 0.0,
        )));

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
          half_width,
          monitor_size.y,
        )));
      }
    }

    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<PositionWindowRightHalfRequest>>()
      .iter(&self.world)
      .next()
    {
      log::info!("[Window] Positioning window to right half");

      self.world.despawn(entity);

      if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
        let half_width = monitor_size.x / 2.0;

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
          half_width, 0.0,
        )));

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
          half_width,
          monitor_size.y,
        )));
      }
    }

    codeshow::poll_pending(&mut self.world, delta);

    // Must run BEFORE voice sync so VoiceToggleCommand is processed first.
    self.schedule.run(&mut self.world);

    let current_state = self
      .world
      .get_resource::<VoiceResource>()
      .map(|v| v.state)
      .unwrap_or(VoiceState::Idle);

    if let Some(voice_manager) = self.voice_manager.as_mut() {
      voice_manager.try_restore_transcriber();

      if current_state != self.prev_voice_state {
        match (self.prev_voice_state, current_state) {
          (VoiceState::Idle, VoiceState::Listening) => {
            if let Some(err) = voice_manager.start_listening().err() {
              log::error!("[Voice] Failed to start listening: {err}");

              if let Some(mut voice) =
                self.world.get_resource_mut::<VoiceResource>()
              {
                voice.set_error(err.to_string());
              }
            }
          }
          (VoiceState::Listening, VoiceState::Idle) => {
            voice_manager.stop_listening();
          }
          _ => {}
        }
      }

      let status = voice_manager.get_status();
      let waveform = voice_manager.get_waveform();

      if let Some(mut voice_res) =
        self.world.get_resource_mut::<VoiceResource>()
      {
        voice_res.waveform = waveform;

        voice_res.set_visualizer_status(status);

        match status {
          VisualizerStatus::Processing
            if voice_res.state == VoiceState::Listening =>
          {
            voice_res.set_state(VoiceState::Processing);
          }
          VisualizerStatus::Success => {
            voice_res.set_state(VoiceState::Idle);
          }
          VisualizerStatus::Error if voice_res.state != VoiceState::Idle => {
            voice_res.set_error("Voice processing failed");
          }
          _ => {}
        }
      }

      if matches!(status, VisualizerStatus::Error)
        && !matches!(self.prev_visualizer_status, VisualizerStatus::Error)
        && let Some(pos) = ctx.input(|i| i.viewport().outer_rect.map(|r| r.min))
      {
        let current_time = ctx.input(|i| i.time);

        self.shake_animation =
          Some(ShakeAnimation::new(current_time, pos.x, pos.y));
      }

      self.prev_visualizer_status = status;
    }

    self.prev_voice_state = current_state;

    let center_step = self
      .center_animation
      .as_ref()
      .map(|anim| anim.tick(ctx.input(|i| i.time)));

    if let Some(step) = center_step {
      ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
        step.x, step.y,
      )));

      if step.finished {
        self.center_animation = None;
        if let Some(mut active) =
          self.world.get_resource_mut::<ActiveAnimations>()
        {
          active.decrement();
        }
      }
    }

    let shake_step = self
      .shake_animation
      .as_ref()
      .map(|anim| anim.tick(ctx.input(|i| i.time)));

    if let Some(step) = shake_step {
      ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
        step.x, step.y,
      )));

      if step.finished {
        self.shake_animation = None;
        if let Some(mut active) =
          self.world.get_resource_mut::<ActiveAnimations>()
        {
          active.decrement();
        }
      }
    }

    if self.shake_animation.is_some()
      && let Some(mut anim) =
        self.world.get_resource_mut::<ContinuousAnimations>()
    {
      anim.set_shake_active();
    }

    voice::tick_continuous_animation(&mut self.world);

    let visuals = assets::theme::get_animated_visuals(&self.world);
    ctx.set_visuals(visuals);

    {
      if !self.html_preview_handle_set
        && let Ok(window_handle) = frame.window_handle()
      {
        self
          .html_preview_webview
          .set_window_handle(window_handle.as_raw());

        self.html_preview_handle_set = true;

        log::debug!("[HtmlPreview] Window handle set");
      }

      let (enabled, rect, needs_reload, current_file) = self
        .world
        .get_resource::<HtmlPreviewState>()
        .map(|s| {
          (
            s.enabled,
            s.webview_rect,
            s.needs_reload,
            s.current_file.clone(),
          )
        })
        .unwrap_or((false, None, false, None));

      if enabled && !self.html_preview_webview.visible {
        self.html_preview_webview.show();
        self.html_preview_webview.try_create_webview();

        log::debug!("[HtmlPreview] WebView shown");
      } else if !enabled && self.html_preview_webview.visible {
        self.html_preview_webview.hide();
        log::debug!("[HtmlPreview] WebView hidden");
      }

      if enabled && needs_reload {
        if let Some(file_path) = &current_file {
          let path_str = file_path.to_string_lossy().to_string();

          self.sdk.send_html_preview_file(path_str);
          self.html_preview_webview.reload();

          log::debug!("[HtmlPreview] Updated preview file: {file_path:?}");
        }

        if let Some(mut s) = self.world.get_resource_mut::<HtmlPreviewState>() {
          s.needs_reload = false;
        }
      }

      if enabled && let Some(r) = rect {
        self
          .html_preview_webview
          .update_bounds(r.x, r.y, r.width, r.height);
      }
    }

    {
      if !self.playground_handle_set
        && let Ok(window_handle) = frame.window_handle()
      {
        self
          .playground_webview
          .set_window_handle(window_handle.as_raw());

        self.playground_handle_set = true;

        log::debug!("[PlaygroundPreview] Window handle set");
      }

      let (enabled, rect, needs_reload) = self
        .world
        .get_resource::<PlaygroundWebviewState>()
        .map(|s| (s.enabled, s.webview_rect, s.needs_reload))
        .unwrap_or((false, None, false));

      if enabled && !self.playground_webview.visible {
        self.playground_webview.show();
        self.playground_webview.try_create_webview();
      } else if !enabled && self.playground_webview.visible {
        self.playground_webview.hide();
      }

      if enabled && needs_reload {
        self.playground_webview.reload();
        log::debug!("[PlaygroundPreview] WebView reloaded");

        if let Some(mut webview_state) =
          self.world.get_resource_mut::<PlaygroundWebviewState>()
        {
          webview_state.needs_reload = false;
        }
      }

      if enabled && let Some(r) = rect {
        self
          .playground_webview
          .update_bounds(r.x, r.y, r.width, r.height);
      }
    }

    if animation::end_frame(&mut self.world) {
      ctx.request_repaint();
    }
  }

  fn save(&mut self, storage: &mut dyn eframe::Storage) {
    if self.clear_session_on_save {
      crate::session::clear_session(storage);

      return;
    }

    let session = crate::session::SessionState::from_world(&mut self.world);

    eframe::set_value(storage, crate::session::SESSION_KEY, &session);

    log::info!(
      "[Session] Saved: {} tabs, active: {:?}",
      session.tabs.len(),
      session.active_tab_index
    );
  }
}

impl Coder {
  fn show(&mut self, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    // Border + corner masks share a dedicated layer that
    // `magic_zoom::propagate` skips, so they stay at 1× during zoom.
    let content_rect = ctx.input(|i| i.viewport_rect());
    let border_color = egui::Color32::from_gray(30);
    let border_painter =
      ctx.layer_painter(effects::magic_zoom::app_border_layer_id());

    // Only mask the rounded-corner bites when zoomed — at identity,
    // leaving them transparent lets the OS background show through.
    if effects::magic_zoom::transform(&self.world).is_some() {
      effects::magic_zoom::mask_corners(
        &border_painter,
        content_rect,
        10.0,
        ctx.global_style().visuals.window_fill,
      );
    }

    border_painter.rect_stroke(
      content_rect,
      10.0,
      egui::Stroke::new(1.0_f32, border_color),
      egui::StrokeKind::Middle,
    );

    egui::Panel::top("titlebar")
      .exact_size(28.0)
      .frame(
        egui::Frame::NONE
          .corner_radius(radius::symmetric(10, 0))
          .fill(ctx.global_style().visuals.window_fill),
      )
      .show_separator_line(false)
      .show_inside(ui, |ui| {
        organisms::titlebar::show(ui, &mut self.world);
        structure::progress_separator::show(ui, &mut self.world);
      });

    {
      let (search_visible, query_empty) = self
        .world
        .get_resource::<SearchState>()
        .map(|s| (s.visible, s.query.is_empty()))
        .unwrap_or((false, true));

      egui::Panel::top("search_panel")
        .resizable(false)
        .exact_size(50.0)
        .frame(egui::Frame::NONE.fill(ctx.global_style().visuals.window_fill))
        .show_animated_inside(ui, search_visible, |ui| {
          panels::search::show(ui, &mut self.world);
        });

      // `show_animated_inside` skips its body when hidden, so the
      // hint animation has to be marked active from out here.
      if search_visible
        && query_empty
        && let Some(mut anim) =
          self.world.get_resource_mut::<ContinuousAnimations>()
      {
        anim.set_search_hint_active();
      }
    }

    egui::Panel::bottom("statusbar")
      .exact_size(28.0)
      .frame(
        egui::Frame::NONE
          .corner_radius(radius::symmetric(0, 10))
          .fill(ctx.global_style().visuals.window_fill)
          .inner_margin(egui::Margin::symmetric(8, 0)),
      )
      .show_inside(ui, |ui| organisms::statusbar::show(ui, &mut self.world));

    let music_player_height = self
      .world
      .get_resource::<MusicPlayerState>()
      .map(|s| s.height_animation.current_value())
      .unwrap_or(40.0);

    egui::Panel::bottom("music_player")
      .exact_size(music_player_height)
      .frame(
        egui::Frame::NONE
          .fill(ctx.global_style().visuals.window_fill)
          .inner_margin(egui::Margin::symmetric(0, 0)),
      )
      .show_inside(ui, |ui| {
        let rect = ui.max_rect();
        let separator_y = rect.top();

        let audio = self
          .world
          .get_resource::<AudioDispatcher>()
          .copied()
          .unwrap_or_default();

        let snapshot = audio.music_snapshot();

        let (progress_ratio, total_width) = if let Some(ref snap) = snapshot {
          let position_secs = snap.position().as_secs_f32();

          let duration_secs =
            snap.duration().map(|d| d.as_secs_f32()).unwrap_or(0.0);

          let ratio = if duration_secs > 0.0 {
            (position_secs / duration_secs).clamp(0.0, 1.0)
          } else {
            0.0
          };
          (ratio, rect.width())
        } else {
          (0.0, rect.width())
        };

        if snapshot.is_some() {
          let progress_bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left(), separator_y - 6.0),
            egui::vec2(total_width, 10.0),
          );

          let progress_response = ui.interact(
            progress_bar_rect,
            ui.id().with("music_progress_bar"),
            egui::Sense::click(),
          );

          if progress_response.clicked()
            && let Some(click_pos) = progress_response.interact_pointer_pos()
          {
            let click_x = click_pos.x - rect.left();
            let seek_ratio = (click_x / total_width).clamp(0.0, 1.0);

            if let Some(ref snap) = snapshot
              && let Some(duration) = snap.duration()
            {
              let seek_position = duration.mul_f32(seek_ratio);

              log::debug!(
                "Seeking to: {seek_position:?} (ratio: {seek_ratio:.2})",
              );

              audio.music_seek(seek_position);
            }
          }

          if progress_response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
          }
        }

        ui.painter().line_segment(
          [
            egui::pos2(rect.left(), separator_y),
            egui::pos2(rect.left() + total_width, separator_y),
          ],
          egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 60, 60)),
        );

        let progress_width = total_width * progress_ratio;
        ui.painter().line_segment(
          [
            egui::pos2(rect.left(), separator_y),
            egui::pos2(rect.left() + progress_width, separator_y),
          ],
          egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(204, 253, 62)),
        );

        if let Some(ref snap) = snapshot
          && let Some(mut state) =
            self.world.get_resource_mut::<MusicPlayerState>()
        {
          state.is_playing =
            snap.state == codelord_audio::PlaybackState::Playing;
        }

        panels::music_player::show(ui, &mut self.world);
      });

    let zoom_margin = self
      .world
      .get_resource::<tabbar::ZoomState>()
      .map(|state| {
        state
          .transition
          .as_ref()
          .map(|t| t.animated_margin)
          .unwrap_or(if state.is_zoomed { 4.0 } else { 0.0 })
      })
      .unwrap_or(0.0);

    let central_frame = egui::Frame::NONE
      .fill(if zoom_margin > 0.0 {
        egui::Color32::WHITE
      } else {
        ctx.global_style().visuals.window_fill
      })
      .inner_margin(egui::Margin::same(zoom_margin.round() as i8));

    egui::CentralPanel::default()
      .frame(central_frame)
      .show_inside(ui, |ui| base::show(ui, &mut self.world));

    overlays::popup::show(&ctx, &mut self.world);

    let filescope_response = overlays::filescope::show(&ctx, &mut self.world);

    filescope::apply_response(&mut self.world, filescope_response);

    let unsaved_response =
      overlays::unsaved_changes_dialog::show(&ctx, &mut self.world);

    tabbar::apply_unsaved_changes_response(&mut self.world, unsaved_response);

    egui::Area::new(egui::Id::new("toaster_overlay"))
      .order(egui::Order::Foreground)
      .anchor(egui::Align2::RIGHT_TOP, egui::vec2(0.0, 0.0))
      .interactable(true)
      .show(&ctx, |ui| {
        let result = overlays::toaster::show(ui, &mut self.world);

        for id in result.dismissed_ids {
          self.world.write_message(DismissToastCommand(id));
        }

        for event in result.action_events {
          self.handle_toast_action(&event.action_id);
        }
      });

    // Overlays render as top-level `egui::Area`s, outside the main
    // body's transform wrap — propagate the zoom so they zoom with it.
    effects::magic_zoom::propagate(&ctx, &self.world);
    voice::check_model_toast(&mut self.world);

    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::T)) {
      self.world.write_message(ThemeCommand {
        action: ThemeAction::Toggle,
      });
    }

    if ctx.input(|i| {
      i.modifiers.command
        && i.modifiers.shift
        && i.key_pressed(egui::Key::OpenBracket)
    }) {
      self.world.spawn(NavigatePrevTabRequest);
    }

    if ctx.input(|i| {
      i.modifiers.command
        && i.modifiers.shift
        && i.key_pressed(egui::Key::CloseBracket)
    }) {
      self.world.spawn(NavigateNextTabRequest);
    }

    if ctx.input(|i| {
      i.modifiers.command
        && i.modifiers.shift
        && i.key_pressed(egui::Key::Space)
    }) {
      self.world.write_message(VoiceToggleCommand);
    }

    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
      use ecs::query::With;

      let active_editor = self
        .world
        .query_filtered::<ecs::entity::Entity, (With<EditorTab>, With<Active>)>(
        )
        .iter(&self.world)
        .next();

      if let Some(entity) = active_editor {
        self.world.spawn(SaveFileRequest::new(entity));
      }
    }

    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
      self.world.spawn(ToggleSearchRequest);
    }

    if ctx.input(|i| {
      i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::G)
    }) {
      self.world.spawn(ToggleBlameRequest);
    }

    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P)) {
      self
        .world
        .resource_mut::<FilescopeState>()
        .toggle(FilescopeMode::Files);
    }

    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::M)) {
      let time = self
        .world
        .get_resource::<DeltaTime>()
        .map(|t| t.elapsed())
        .unwrap_or(0.0);

      self
        .world
        .resource_mut::<MusicPlayerState>()
        .toggle_visibility(time);
    }

    // Held-key, not toggle: matches the "held-key modality" doctrine
    // (like Cmd+Shift+Space for voice). Emit the command only on
    // transition to avoid spamming Messages every frame; retarget the
    // camera center each frame while held so the zoom follows the cursor.
    let (want_engage, cursor) = ctx.input(|i| {
      let held = i.modifiers.command && i.key_down(egui::Key::E);
      let cursor = i.pointer.hover_pos().map(|p| (p.x, p.y));

      (held, cursor)
    });

    let was_engaged = self.world.resource::<MagicZoomState>().engaged;

    if was_engaged != want_engage {
      self.world.write_message(MagicZoomCommand {
        engage: want_engage,
      });
    } else if want_engage && let Some((x, y)) = cursor {
      self
        .world
        .resource_mut::<MagicZoomState>()
        .retarget_center(x, y);
    }
  }

  /// Handle toast action button clicks.
  fn handle_toast_action(&mut self, action_id: &str) {
    match action_id {
      "voice_download" => {
        log::info!("[Voice] Starting model download from toast action");

        if let Some(mut model_state) =
          self.world.get_resource_mut::<VoiceModelState>()
        {
          model_state.start_download();
        }

        if let Some(mut loading) =
          self.world.get_resource_mut::<GlobalLoading>()
        {
          loading.start(LoadingTask::Network);
        }

        let download_rx = codelord_sdk::voice::spawn_download();

        self.voice_model_download_rx = Some(download_rx);
      }
      _ => {
        log::debug!("[Toast] Unknown action: {action_id}");
      }
    }
  }

  /// Restore session state from storage.
  /// Returns true if tabs were restored (session had open files).
  fn restore_session(
    cc: &eframe::CreationContext<'_>,
    world: &mut World,
  ) -> bool {
    let Some(storage) = cc.storage else {
      log::debug!("[Session] No storage available");

      return false;
    };

    let Some(session) = eframe::get_value::<crate::session::SessionState>(
      storage,
      crate::session::SESSION_KEY,
    ) else {
      log::debug!("[Session] No saved session found");

      return false;
    };

    session.apply_to_world(world)
  }
}
