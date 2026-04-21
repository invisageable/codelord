use codelord_components::assets;
use codelord_components::components::effects;
use codelord_components::components::indicators::frame_history;
use codelord_components::components::layouts::base;
use codelord_components::components::organisms::{statusbar, titlebar};
use codelord_components::components::overlays;
use codelord_components::components::panels::music_player;
use codelord_components::components::panels::search as search_panel;
use codelord_components::components::renderers::svg::SvgTextureCacheResource;
use codelord_core::animation::components::DeltaTime;
use codelord_core::animation::resources::{
  ActiveAnimations, ContinuousAnimations,
};
use codelord_core::audio;
use codelord_core::audio::resources::{MusicPlayerState, Playlist};
use codelord_core::codeshow::{CodeshowState, NavigateSlide, SlideDirection};
use codelord_core::drag_and_drop::{DragAnimationState, DragOrder, DragState};
use codelord_core::ecs::message::Messages;
use codelord_core::ecs::schedule::{IntoScheduleConfigs, Schedule};
use codelord_core::ecs::world::World;
use codelord_core::events::{
  CenterWindowRequest, ClearSessionRequest, CompileRequest,
  NavigateNextTabRequest, NavigatePrevTabRequest, OpenFileRequest,
  OpenPdfPreviewRequest, PositionWindowLeftHalfRequest,
  PositionWindowRightHalfRequest, SaveFileRequest, ShakeWindowRequest,
  ToggleBlameRequest, ToggleSearchRequest,
};
use codelord_core::icon::components::{
  Icon, StatusbarIconBundle, TitlebarIconBundle,
};
use codelord_core::keyboard;
use codelord_core::keyboard::{Focusable, KeyboardFocus, KeyboardHandler};
use codelord_core::loading::{GlobalLoading, LoadingTask};
use codelord_core::magic_zoom::{
  MagicZoomCommand, MagicZoomState, update_magic_zoom_system,
};
use codelord_core::navigation;
use codelord_core::navigation::resources::{
  ActiveWorkspaceRoot, BreadcrumbData, ExplorerContextTarget,
  ExplorerEditingState, ExplorerItemsCounter, ExplorerState, FileClipboard,
  IndentationLinesState, StagebarResource,
};
use codelord_core::page;
use codelord_core::page::resources::{
  PageResource, PageSwitchCommand, PageSwitchEvent,
};
use codelord_core::panel;
use codelord_core::panel::resources::{
  BottomPanelResource, LeftPanelResource, PanelCommand, RightPanelResource,
};
use codelord_core::playground;
use codelord_core::runtime::RuntimeHandle;
use codelord_core::search::SearchState;
use codelord_core::tabbar::components::EditorTab;

use codelord_core::about::resources::AboutResource;
use codelord_core::animation::resources::ShakeAnimation;
use codelord_core::filescope::resources::{
  FilescopeMatcher, FilescopeMode, FilescopeResponse, FilescopeState,
};
use codelord_core::git::resources::{
  GitBlameSettings, GitBranchState, PendingBlameRequests, PendingBranchRequests,
};
use codelord_core::instruction::resources::InstructionsResource;
use codelord_core::playground::{
  FeedbackState, PLAYGROUND_PREVIEW_URL, PlaygroundFeedback,
  PlaygroundHoveredSpan, PlaygroundMetrics, PlaygroundOutput,
  PlaygroundWebviewState,
};
use codelord_core::popup;
use codelord_core::popup::components::{
  MenuItem, Popup, PopupContent, PopupPosition,
};
use codelord_core::popup::resources::{PopupCommand, PopupResource};
use codelord_core::previews::sqlite::SqliteQuery;
use codelord_core::previews::{
  CsvPreviewState, DEFAULT_PREVIEW_URL, FontPreviewState, HtmlPreviewState,
  MarkdownPreviewState, PdfConnection, PdfPageCache, PdfPreviewState,
  PdfSelection, PdfTextCache, SqliteConnection, SqlitePreviewState,
  SvgPreviewState, XlsPreviewState, close_pdf_connection_system,
  close_sqlite_connection_system, dispatch_pdf_queries_system,
  dispatch_sqlite_queries_system, poll_pdf_results_system,
  poll_sqlite_results_system,
};
use codelord_core::settings::resources::SettingsResource;
use codelord_core::statusbar::resources::{
  LineColumnAnimation, StatusbarResource,
};
use codelord_core::statusbar::systems::line_column_animation_system;
use codelord_core::tabbar::components::{PlaygroundTab, SonarAnimation, Tab};
use codelord_core::tabbar::{
  self, TabContextTarget, TabOrderCounter, UnsavedChangesDialog,
  UnsavedChangesResponse, ZoomState,
};
use codelord_core::terminal;
use codelord_core::terminal::{
  TerminalBridges, TerminalIdCounter, TerminalRegistry, TerminalTabOrderCounter,
};
use codelord_core::text_editor;
use codelord_core::text_editor::components::FileTab;
use codelord_core::text_editor::components::{Cursor, TextBuffer};
use codelord_core::theme;
use codelord_core::theme::components::ThemeKind;
use codelord_core::theme::resources::{
  ThemeAction, ThemeChangedEvent, ThemeCommand, ThemeResource,
};
use codelord_core::toast;
use codelord_core::toast::components::ToastAction;
use codelord_core::toast::resources::{
  DismissToastCommand, ToastCommand, ToasterResource,
};
use codelord_core::ui::component::{Active, Metric};
use codelord_core::ui::component::{DecorationBundle, DecorationType};
use codelord_core::voice;
use codelord_core::voice::components::VoiceState;
use codelord_core::voice::resources::{
  ModelStatus, VisualizerStatus, VoiceActionEvent, VoiceModelState,
  VoiceResource, VoiceToggleCommand,
};
use codelord_core::xmb;
use codelord_core::xmb::resources::{XmbCommand, XmbResource};
use codelord_protocol::compilation::CompilationEvent;
use codelord_protocol::event::ServerEvent;
use codelord_protocol::voice::model::VoiceAction;
use codelord_sdk::Sdk;
use codelord_voice::VoiceManager;

use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::HasWindowHandle;
#[cfg(not(target_arch = "wasm32"))]
use swisskit::renderer::html::HtmlRenderer;

use std::sync::Arc;

/// Animation state for smooth window centering.
struct CenterWindowAnimation {
  start_time: f64,
  duration: f64,
  start_pos: egui::Pos2,
  end_pos: egui::Pos2,
}

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
  voice_action_rx: flume::Receiver<VoiceAction>,
  /// Previous voice state (to detect transitions)
  prev_voice_state: VoiceState,
  /// Voice model download receiver (when download is in progress)
  voice_model_download_rx:
    Option<flume::Receiver<codelord_sdk::voice::DownloadResult>>,
  /// Previous visualizer status (to detect error transitions)
  prev_visualizer_status: VisualizerStatus,
  /// Shake animation for error feedback
  shake_animation: Option<ShakeAnimation>,
  /// Center window animation
  center_animation: Option<CenterWindowAnimation>,
  /// HTML preview WebView (stored outside ECS because wry::WebView is
  /// !Send+!Sync)
  #[cfg(not(target_arch = "wasm32"))]
  html_preview_webview: HtmlRenderer,
  /// Whether the window handle has been set for the HTML preview WebView
  #[cfg(not(target_arch = "wasm32"))]
  html_preview_handle_set: bool,
  /// Playground WebView for templating mode (stored outside ECS)
  #[cfg(not(target_arch = "wasm32"))]
  playground_webview: HtmlRenderer,
  /// Whether the window handle has been set for the playground WebView
  #[cfg(not(target_arch = "wasm32"))]
  playground_handle_set: bool,
  /// Flag to clear session on next save (instead of saving)
  clear_session_on_save: bool,
  /// Channel to receive compilation events from server
  compilation_event_rx: Option<flume::Receiver<CompilationEvent>>,
  /// Gilrs gamepad/remote control input handler
  #[cfg(not(target_arch = "wasm32"))]
  gilrs: Option<gilrs::Gilrs>,
}

impl Coder {
  /// Create a new IDE application
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    effects::wave::WaveCallback::init(cc);

    let mut world = World::new();

    // ========================================================================
    // Initialize Resources
    // ========================================================================

    // Theme system
    world.insert_resource(ThemeResource::new(ThemeKind::Dark));

    // Initialize message queues
    world.init_resource::<Messages<ThemeCommand>>();
    world.init_resource::<Messages<ThemeChangedEvent>>();

    // Page system
    world.insert_resource(PageResource::default());
    world.init_resource::<Messages<PageSwitchCommand>>();
    world.init_resource::<Messages<PageSwitchEvent>>();

    // Initialize delta time resource (updated each frame)
    world.insert_resource(DeltaTime::default());

    // Initialize active animations counter
    world.insert_resource(ActiveAnimations::default());

    // Initialize continuous animations tracker
    world.insert_resource(ContinuousAnimations::default());

    // Global loading indicator
    world.insert_resource(GlobalLoading::default());

    world.insert_resource(ExplorerItemsCounter::default());
    world.insert_resource(ExplorerState::default());
    world.insert_resource(ActiveWorkspaceRoot::default());
    world.insert_resource(BreadcrumbData::default());
    world
      .insert_resource(codelord_core::symbol::StickyScrollSettings::default());
    world.insert_resource(
      codelord_core::text_editor::resources::IndentGuidesSettings::default(),
    );
    world.insert_resource(IndentationLinesState::default());
    world.insert_resource(ExplorerContextTarget::default());
    world.insert_resource(ExplorerEditingState::default());
    world.insert_resource(FileClipboard::default());

    // Symbol extractors (registered from codelord-language)
    world.insert_resource(
      codelord_core::symbol::resources::SymbolExtractors::new()
        .register(
          codelord_core::language::Language::C,
          codelord_language::language::c::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Elixir,
          codelord_language::language::elixir::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::JavaScript,
          codelord_language::language::javascript::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Json,
          codelord_language::language::json::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Python,
          codelord_language::language::python::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Rust,
          codelord_language::language::rust::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Zig,
          codelord_language::language::zig::symbols::extract_symbols,
        )
        .register(
          codelord_core::language::Language::Zo,
          codelord_language::language::zo::symbols::extract_symbols,
        ),
    );

    // Token extractors for syntax highlighting (registered from
    // codelord-language)
    world.insert_resource(
      codelord_core::token::TokenExtractors::new()
        .register(
          codelord_core::language::Language::Bash,
          codelord_language::language::bash::highlights::parse,
        )
        .register(
          codelord_core::language::Language::C,
          codelord_language::language::c::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Conf,
          codelord_language::language::conf::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Css,
          codelord_language::language::css::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Diff,
          codelord_language::language::diff::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Elixir,
          codelord_language::language::elixir::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Env,
          codelord_language::language::env::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Gleam,
          codelord_language::language::gleam::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Go,
          codelord_language::language::go::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Html,
          codelord_language::language::html::highlights::parse,
        )
        .register(
          codelord_core::language::Language::JavaScript,
          codelord_language::language::javascript::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Json,
          codelord_language::language::json::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Markdown,
          codelord_language::language::markdown::highlights::parse,
        )
        // .register(
        //   codelord_core::language::Language::Ocaml,
        //   codelord_language::language::ocaml::highlights::parse,
        // )
        .register(
          codelord_core::language::Language::Python,
          codelord_language::language::python::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Rust,
          codelord_language::language::rust::highlights::parse,
        )
        .register(
          codelord_core::language::Language::TypeScript,
          codelord_language::language::typescript::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Yaml,
          codelord_language::language::yaml::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Zig,
          codelord_language::language::zig::highlights::parse,
        )
        .register(
          codelord_core::language::Language::Zo,
          codelord_language::language::zo::highlights::parse,
        ),
    );

    // Color detection for inline color previews
    world.insert_resource(codelord_core::color::ColorCache::new());
    world.insert_resource(codelord_core::color::ColorExtractor::new(
      codelord_language::color::extract,
    ));
    world.insert_resource(codelord_core::color::ColorPickerState::default());

    // Statusbar resources
    world.insert_resource(StatusbarResource::new());
    world.insert_resource(LineColumnAnimation::new());

    // Panel resources
    world.insert_resource(LeftPanelResource::default());
    world.insert_resource(RightPanelResource::default());
    world.insert_resource(BottomPanelResource::default());
    world.init_resource::<Messages<PanelCommand>>();

    // Audio resources
    world.insert_resource(MusicPlayerState::new());
    world.insert_resource(Playlist::new());

    // Initialize audio system (spawns dedicated audio thread).
    if let Err(e) = audio::init() {
      log::error!("Failed to initialize audio system: {e}");
    }

    // HTML Preview state resource (WebView stored outside ECS)
    world.insert_resource(HtmlPreviewState::default());

    // Markdown Preview state resource
    world.insert_resource(MarkdownPreviewState::default());

    // CSV Preview state resource
    world.insert_resource(CsvPreviewState::default());

    // PDF Preview state resource
    world.insert_resource(PdfPreviewState::default());
    world.insert_resource(PdfConnection::default());
    world.insert_resource(PdfPageCache::default());
    world.insert_resource(PdfTextCache::default());
    world.insert_resource(PdfSelection::default());

    // SQLite Preview state resource
    world.insert_resource(SqlitePreviewState::new());
    world.insert_resource(SqliteConnection::default());

    // XLS Preview state resource
    world.insert_resource(XlsPreviewState::default());

    // Font Preview state resource
    world.insert_resource(FontPreviewState::default());

    // SVG Preview state resource
    world.insert_resource(SvgPreviewState::default());
    world.insert_non_send_resource(SvgTextureCacheResource::default());

    // Search state resource
    world.insert_resource(SearchState::default());

    // Popup resources
    world.insert_resource(PopupResource::new());
    world.init_resource::<Messages<PopupCommand>>();

    // Tab order counter
    world.insert_resource(TabOrderCounter::default());

    // Zoom state
    world.insert_resource(ZoomState::default());

    // Drag and drop state
    world.insert_resource(DragState::default());
    world.insert_resource(DragAnimationState::default());

    // Tab context target (for right-click menu)
    world.insert_resource(TabContextTarget::default());

    // Unsaved changes dialog
    world.insert_resource(UnsavedChangesDialog::default());

    // Keyboard focus
    world.insert_resource(KeyboardFocus::new());

    // XMB resources (for welcome screen)
    world.insert_resource(XmbResource::new());
    world.init_resource::<Messages<XmbCommand>>();

    // Magic zoom (Screen-Studio-style camera effect)
    world.insert_resource(MagicZoomState::default());
    world.init_resource::<Messages<MagicZoomCommand>>();

    // About resources
    world.insert_resource(AboutResource::default());

    // Settings resources
    world.insert_resource(SettingsResource::default());

    // Git resources
    world.insert_resource(GitBlameSettings::default());
    world.insert_resource(PendingBlameRequests::default());
    world.insert_resource(GitBranchState::default());
    world.insert_resource(PendingBranchRequests::default());

    // Instructions resource (for empty editor state)
    world.insert_resource(InstructionsResource::default());

    // Toast resources
    world.insert_resource(ToasterResource::default());
    world.init_resource::<Messages<ToastCommand>>();
    world.init_resource::<Messages<DismissToastCommand>>();

    // Terminal resources
    world.insert_resource(TerminalIdCounter::default());
    world.insert_resource(TerminalTabOrderCounter::default());
    world.insert_resource(TerminalRegistry::default());
    world.insert_resource(TerminalBridges::default());

    // Voice resources
    world.insert_resource(VoiceResource::default());
    world.insert_resource(VoiceModelState::default());
    world.init_resource::<Messages<VoiceToggleCommand>>();
    world.init_resource::<Messages<VoiceActionEvent>>();

    // Filescope resources
    world.insert_resource(FilescopeState::default());
    world.insert_resource(FilescopeMatcher::new());

    // Codeshow (presenter) resources
    world.insert_resource(codelord_core::codeshow::CodeshowState::default());

    // ========================================================================
    // Initialize Async Runtime & Voice System
    // ========================================================================

    let runtime = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(2)
      .enable_all()
      .build()
      .expect("Failed to create Tokio runtime");

    world.insert_resource(RuntimeHandle::new(runtime.handle().clone()));

    let sdk = Arc::new(Sdk::new(runtime.handle().clone()));

    let (voice_action_tx, voice_action_rx) = flume::unbounded::<VoiceAction>();

    // Always insert VoiceVisualizerState (even if VoiceManager fails)
    let visualizer_state = codelord_voice::VoiceVisualizerState::new();
    world.insert_resource(visualizer_state.clone());

    let voice_manager = VoiceManager::new(
      voice_action_tx,
      None,
      runtime.handle().clone(),
      sdk.clone(),
      visualizer_state,
    )
    .map_err(|e| {
      log::warn!("Voice manager initialization failed: {e}");
      e
    })
    .ok();

    voice_manager.as_ref().map(|vm| {
      world
        .get_resource_mut::<VoiceResource>()
        .map(|mut voice_res| voice_res.is_available = vm.is_available())
    });

    // Check voice model status on startup
    if let Some(mut model_state) = world.get_resource_mut::<VoiceModelState>() {
      if codelord_voice::transcriber::model_exists() {
        model_state.set_ready();
        log::info!(
          "[Voice] Model found at: {}",
          codelord_voice::transcriber::model_path().display()
        );
      } else {
        model_state.status = ModelStatus::Missing;
        log::info!("[Voice] Model not found, will prompt on first use");
      }
    }

    // ========================================================================
    // Spawn Initial Entities
    // ========================================================================

    // Spawn window decoration buttons
    world.spawn(DecorationBundle::new(DecorationType::Close));
    world.spawn(DecorationBundle::new(DecorationType::MinimizeMaximize));
    world.spawn(DecorationBundle::new(DecorationType::Fullscreen));

    // Spawn titlebar icon buttons with drag order
    world.spawn((TitlebarIconBundle::new(Icon::Home), DragOrder(0)));
    world.spawn((TitlebarIconBundle::new(Icon::Code), DragOrder(1)));
    world.spawn((TitlebarIconBundle::new(Icon::Ufo), DragOrder(2)));
    world.spawn((TitlebarIconBundle::new(Icon::Alien), DragOrder(3)));

    // Spawn statusbar icon buttons
    let explorer_btn =
      world.spawn(StatusbarIconBundle::new(Icon::Explorer)).id();

    let voice_btn = world.spawn(StatusbarIconBundle::new(Icon::Voice)).id();

    // Register statusbar buttons in the resource
    if let Some(mut statusbar) = world.get_resource_mut::<StatusbarResource>() {
      statusbar.add_left(explorer_btn);
      statusbar.add_right(voice_btn);
    }

    // Spawn settings popup
    let settings_menu = PopupContent::Menu(vec![
      MenuItem::new("about", "About Codelord"),
      MenuItem::new("settings", "Settings").with_shortcut("Cmd+,"),
      MenuItem::new("check_updates", "Check for Updates"),
    ]);

    let settings_popup = world.spawn(Popup::new(settings_menu)).id();

    // Store settings popup entity in resource
    if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
      popup_res.settings_popup = Some(settings_popup);
    }

    // Spawn explorer context menu popup
    let explorer_context_menu = PopupContent::Menu(vec![
      MenuItem::new("new_file", "New File"),
      MenuItem::new("new_folder", "New Folder").with_separator(),
      MenuItem::new("add_folder_to_workspace", "Add Folder to Workspace"),
      MenuItem::new("remove_from_workspace", "Remove from Workspace")
        .with_separator(),
      MenuItem::new("cut", "Cut"),
      MenuItem::new("copy", "Copy"),
      MenuItem::new("paste", "Paste").with_separator(),
      MenuItem::new("copy_path", "Copy Path"),
      MenuItem::new("copy_relative_path", "Copy Relative Path"),
      MenuItem::new("reveal_in_finder", "Reveal in Finder").with_separator(),
      MenuItem::new("rename", "Rename"),
      MenuItem::new("delete", "Delete"),
    ]);

    let explorer_context_popup = world
      .spawn(
        Popup::new(explorer_context_menu).with_position(PopupPosition::Cursor),
      )
      .id();

    if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
      popup_res.explorer_context_popup = Some(explorer_context_popup)
    }

    // Spawn tab context menu popup
    let tab_context_menu = PopupContent::Menu(vec![
      MenuItem::new("close_tab", "Close"),
      MenuItem::new("close_others", "Close Others"),
      MenuItem::new("close_to_right", "Close to the Right").with_separator(),
      MenuItem::new("close_all", "Close All"),
    ]);

    let tab_context_popup = world
      .spawn(Popup::new(tab_context_menu).with_position(PopupPosition::Cursor))
      .id();

    if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
      popup_res.tab_context_popup = Some(tab_context_popup)
    }

    // Spawn SQLite export popup
    let sqlite_export_menu = PopupContent::Menu(vec![
      MenuItem::new("export_csv", "Export as CSV"),
      MenuItem::new("export_json", "Export as JSON"),
    ]);

    let sqlite_export_popup = world
      .spawn(Popup::new(sqlite_export_menu).with_position(PopupPosition::Below))
      .id();

    if let Some(mut popup_res) = world.get_resource_mut::<PopupResource>() {
      popup_res.sqlite_export_popup = Some(sqlite_export_popup)
    }

    // ========================================================================
    // Restore Session or Create Default Tab
    // ========================================================================

    let session_restored = Self::restore_session(cc, &mut world);

    // Spawn default playground tab only if no session was restored
    if !session_restored {
      let order = world
        .get_resource_mut::<TabOrderCounter>()
        .map(|mut counter| counter.next())
        .unwrap_or(0);

      const DEFAULT_CONTENT: &str = r#"fun main() {
  imu view: </> ::= <>hello world!</>;
  #dom view;
}
"#;

      world.spawn((
        Tab::new("playground-1", order),
        PlaygroundTab,
        SonarAnimation::default(),
        TextBuffer::new(DEFAULT_CONTENT),
        Cursor::new(0),
        FileTab::new("playground-1.zo"), // For zo syntax highlighting
        Active,
        Focusable,
        KeyboardHandler::text_editor(),
      ));
    }

    // Initialize playground metrics
    let output_metric = world
      .spawn(Metric::new(
        "OUTPUT",
        "Total size of the compiled output in bytes.",
        0.0,
        "tokens",
        [255, 255, 255, 255], // White
      ))
      .id();

    let time_metric = world
      .spawn(Metric::new_time(
        "TIME",
        "Total compilation time in milliseconds.",
        0.0,
        [204, 255, 0, 255], // Lime green
      ))
      .id();

    world.insert_resource(PlaygroundMetrics {
      output: Some(output_metric),
      time: Some(time_metric),
    });

    world.insert_resource(PlaygroundFeedback::default());
    world.insert_resource(PlaygroundOutput::default());
    world.insert_resource(PlaygroundHoveredSpan::default());
    world.insert_resource(PlaygroundWebviewState::default());

    // Stagebar for playground output view
    world.insert_resource(StagebarResource::compiler_stages());

    // ========================================================================
    // Setup Systems Schedule
    // ========================================================================

    let mut schedule = Schedule::default();

    // Theme systems - order matters for animation
    schedule.add_systems(
      (
        theme::systems::theme_command_system,
        theme::systems::theme_animation_system,
        theme::systems::theme_animation_update_system,
      )
        .chain(),
    );

    schedule.add_systems((
      theme::systems::theme_change_detection_system,
      theme::systems::theme_overrcodelord_system,
      theme::systems::theme_hot_reload_system,
    ));

    // Page systems - order matters for animation
    schedule.add_systems(
      (
        page::systems::page_switch_command_system,
        page::systems::page_transition_update_system,
      )
        .chain(),
    );

    // Magic zoom camera update (reads DeltaTime, drains MagicZoomCommand).
    schedule.add_systems(update_magic_zoom_system);

    // Navigation systems
    schedule.add_systems((
      navigation::systems::poll_folder_dialog_system,
      navigation::systems::folder_selected_system,
      navigation::systems::scan_directory_system,
      navigation::systems::expand_folder_system,
      navigation::systems::collapse_folder_system,
      navigation::systems::update_breadcrumbs_system,
      navigation::systems::create_file_system,
      navigation::systems::create_folder_system,
      navigation::systems::rename_system,
      navigation::systems::delete_system,
      navigation::systems::paste_system,
      // Workspace multi-root systems
      navigation::systems::add_folder_to_workspace_dialog_system,
      navigation::systems::poll_workspace_folder_dialog_system,
      navigation::systems::add_root_system,
      navigation::systems::remove_root_system,
      // Explorer header action systems
      navigation::systems::refresh_explorer_system,
      navigation::systems::collapse_all_folders_system,
      navigation::systems::toggle_hidden_files_system,
      // Explorer selection sync
      navigation::systems::sync_explorer_selection_system,
    ));

    // Keyboard focus systems
    schedule.add_systems((
      keyboard::systems::focus_request_system,
      keyboard::systems::clear_focus_system,
    ));

    // Text editor systems - process file/tab events and text editing
    schedule.add_systems((
      text_editor::systems::open_file_system,
      text_editor::systems::new_editor_tab_system,
      text_editor::systems::activate_tab_system,
      text_editor::systems::insert_text_system,
      text_editor::systems::delete_text_system,
      text_editor::systems::move_cursor_system,
      text_editor::systems::set_cursor_system,
      text_editor::systems::save_file_system,
      text_editor::systems::save_as_dialog_system,
      text_editor::systems::poll_save_file_dialog_system,
      text_editor::systems::toggle_fold_system,
    ));

    // Symbol extraction system - runs after text editing
    schedule
      .add_systems(codelord_core::symbol::systems::extract_symbols_system);

    // Git systems
    schedule.add_systems((
      codelord_core::git::systems::sync_blame_settings_system,
      codelord_core::git::systems::toggle_blame_system,
      codelord_core::git::systems::fetch_blame_system,
      codelord_core::git::systems::poll_blame_results_system,
      codelord_core::git::systems::invalidate_blame_on_edit_system,
      codelord_core::git::systems::detect_branch_system,
      codelord_core::git::systems::poll_branch_results_system,
      codelord_core::git::systems::check_dirty_status_system,
      codelord_core::git::systems::poll_status_results_system,
    ));

    // Playground systems
    schedule.add_systems((
      playground::systems::new_playground_tab_system,
      playground::systems::activate_playground_tab_system,
    ));

    // Statusbar systems
    schedule.add_systems(line_column_animation_system);

    // Tabbar systems
    schedule.add_systems((
      tabbar::systems::close_editor_tab_system,
      tabbar::systems::close_terminal_tab_system,
      tabbar::systems::close_playground_tab_system,
      tabbar::systems::close_all_editor_tabs_system,
      tabbar::systems::close_other_editor_tabs_system,
      tabbar::systems::close_tabs_to_right_editor_system,
      tabbar::systems::navigate_prev_editor_tab_system,
      tabbar::systems::navigate_next_editor_tab_system,
      tabbar::systems::navigate_prev_terminal_tab_system,
      tabbar::systems::navigate_next_terminal_tab_system,
      tabbar::systems::zoom_toggle_system,
      tabbar::systems::zoom_animation_system,
    ));

    // Panel systems (preview toggles)
    schedule.add_systems((
      panel::systems::panel_command_system,
      panel::systems::toggle_html_preview_system,
      panel::systems::update_html_preview_on_tab_change,
      panel::systems::toggle_markdown_preview_system,
      panel::systems::update_markdown_preview_on_change,
      panel::systems::update_markdown_preview_on_tab_change,
      panel::systems::toggle_csv_preview_system,
      panel::systems::update_csv_preview_on_change,
      panel::systems::update_csv_preview_on_tab_change,
      panel::systems::open_pdf_preview_system,
      panel::systems::close_pdf_preview_system,
      panel::systems::update_pdf_preview_on_tab_change,
    ));

    // SQLite preview systems
    schedule.add_systems((
      panel::systems::toggle_sqlite_preview_system,
      panel::systems::update_sqlite_preview_on_tab_change,
      panel::systems::select_sqlite_table_system,
      panel::systems::change_sqlite_page_system,
      panel::systems::execute_sqlite_sql_system,
      panel::systems::export_sqlite_data_system,
    ));

    // XLS preview systems
    schedule.add_systems((
      panel::systems::select_xls_sheet_system,
      panel::systems::change_xls_page_system,
    ));

    // SVG preview systems
    schedule.add_systems((
      panel::systems::svg_zoom_in_system,
      panel::systems::svg_zoom_out_system,
      panel::systems::svg_zoom_reset_system,
    ));

    // Search systems
    schedule.add_systems((
      panel::systems::toggle_search_system,
      panel::systems::hcodelord_search_system,
      panel::systems::update_search_query_system,
      panel::systems::toggle_search_option_system,
      panel::systems::find_next_system,
      panel::systems::find_previous_system,
      panel::systems::execute_search_system,
    ));

    // Popup systems
    schedule.add_systems(popup::systems::popup_command_system);

    // XMB systems (welcome screen navigation)
    schedule.add_systems((
      xmb::systems::xmb_command_system,
      xmb::systems::xmb_action_system,
    ));

    // Terminal systems
    schedule.add_systems((
      terminal::systems::new_terminal_system,
      terminal::systems::new_terminal_tab_system,
      terminal::systems::close_terminal_system,
      terminal::systems::activate_terminal_system,
    ));

    // Toast systems
    schedule.add_systems((
      toast::systems::process_toast_commands,
      toast::systems::process_dismiss_commands,
      toast::systems::update_toast_animations,
    ));

    // Voice systems
    schedule.add_systems((
      voice::systems::voice_toggle_system,
      voice::systems::voice_animation_system,
      voice::systems::voice_action_system,
    ));

    // Filescope systems
    schedule.add_systems((
      codelord_core::filescope::systems::filescope_populate_system,
      codelord_core::filescope::systems::filescope_tick_system,
    ));

    // SQLite preview systems (poll results, dispatch queries, close connection)
    #[cfg(not(target_arch = "wasm32"))]
    schedule.add_systems((
      poll_sqlite_results_system,
      dispatch_sqlite_queries_system,
      close_sqlite_connection_system,
    ));

    // PDF preview systems (poll results, dispatch queries, close connection)
    #[cfg(not(target_arch = "wasm32"))]
    schedule.add_systems((
      poll_pdf_results_system,
      dispatch_pdf_queries_system,
      close_pdf_connection_system,
    ));

    // ========================================================================
    // Apply Initial Theme
    // ========================================================================

    let initial_theme = assets::theme::get_theme(&world);

    cc.egui_ctx
      .set_visuals(assets::theme::theme_to_visuals(initial_theme));

    // ========================================================================
    // Install egui image loaders (for SVG support)
    // ========================================================================

    assets::install_assets(&cc.egui_ctx);

    // ========================================================================
    // Setup Compilation Event Listener
    // ========================================================================

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
        Err(e) => {
          log::warn!("[Compilation] Failed to connect to events: {e}");
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
      #[cfg(not(target_arch = "wasm32"))]
      html_preview_webview: HtmlRenderer::new(DEFAULT_PREVIEW_URL),
      #[cfg(not(target_arch = "wasm32"))]
      html_preview_handle_set: false,
      #[cfg(not(target_arch = "wasm32"))]
      playground_webview: HtmlRenderer::new(PLAYGROUND_PREVIEW_URL),
      #[cfg(not(target_arch = "wasm32"))]
      playground_handle_set: false,
      clear_session_on_save: false,
      compilation_event_rx: Some(compilation_rx),
      #[cfg(not(target_arch = "wasm32"))]
      gilrs: gilrs::Gilrs::new()
        .map_err(|e| log::warn!("[Gilrs] Failed to initialize: {e}"))
        .ok(),
    }
  }
}

impl Coder {
  /// Layer id of the magic-zoom sublayer wrapping `self.show(ui)`.
  fn magic_zoom_layer_id() -> egui::LayerId {
    egui::LayerId::new(egui::Order::Middle, egui::Id::new("magic_zoom_layer"))
  }

  /// Current camera transform, or `None` if the zoom is effectively 1x.
  /// Returning `None` lets callers skip-wrap on the identity case.
  fn magic_zoom_transform(&self) -> Option<egui::emath::TSTransform> {
    let state = self.world.resource::<MagicZoomState>();
    let zoom = state.zoom();

    if (zoom - 1.0).abs() < 0.001 {
      return None;
    }

    let (cx, cy) = state.center();
    let c = egui::vec2(cx, cy);

    Some(
      egui::emath::TSTransform::from_translation(c)
        * egui::emath::TSTransform::from_scaling(zoom)
        * egui::emath::TSTransform::from_translation(-c),
    )
  }

  /// Propagate the magic-zoom transform to every visible layer except our
  /// own (already transformed in `fn ui`). Overlays — filescope, popups,
  /// dialogs, toasts — render via `egui::Area` outside the `scope_builder`
  /// wrap, so without this they'd stay at 1x while the main body zooms.
  ///
  /// On idle frames we push `TSTransform::IDENTITY` to clear any stale
  /// entries from a just-finished zoom (egui stores transforms across
  /// frames).
  fn propagate_magic_zoom(&self, ctx: &egui::Context) {
    let magic_id = Self::magic_zoom_layer_id();
    let transform = self
      .magic_zoom_transform()
      .unwrap_or(egui::emath::TSTransform::IDENTITY);

    let layer_ids: Vec<egui::LayerId> = ctx.memory(|m| m.layer_ids().collect());

    for id in layer_ids {
      if id == magic_id {
        continue;
      }

      ctx.set_transform_layer(id, transform);
    }
  }
}

impl eframe::App for Coder {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
  }

  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    // Magic zoom: wrap the entire app body (titlebar, search, central,
    // statusbar, music player) in a transformed layer when active.
    // Overlays are handled separately in `fn logic` via
    // `propagate_magic_zoom` — they render as top-level `Area`s and need
    // their own transform pass. Skip-wrap keeps the identity case
    // zero-cost.
    let transform = self.magic_zoom_transform();

    egui::CentralPanel::default()
      .frame(egui::Frame::NONE)
      .show_inside(ui, |ui| {
        let Some(transform) = transform else {
          self.show(ui);
          return;
        };

        let layer_id = Self::magic_zoom_layer_id();
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

    // Update delta time resource for ECS systems
    // ========================================================================
    if let Some(mut dt) = self.world.get_resource_mut::<DeltaTime>() {
      dt.update(delta);
    }

    // ========================================================================
    // Poll Voice Actions (from async dispatcher)
    // ========================================================================

    while let Ok(voice_action) = self.voice_action_rx.try_recv() {
      log::info!("[Voice] Received action: {}", voice_action.action);
      self.world.write_message(VoiceActionEvent {
        action: voice_action.action,
        payload: voice_action.payload,
      });
    }

    // ========================================================================
    // Poll Voice Model Download (if in progress)
    // ========================================================================

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

            // Finish global loading indicator
            if let Some(mut loading) =
              self.world.get_resource_mut::<GlobalLoading>()
            {
              loading.finish(LoadingTask::Network);
            }

            // Load transcriber into VoiceManager
            if let Some(ref mut vm) = self.voice_manager
              && vm.load_transcriber()
            {
              // Update VoiceResource availability
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
          codelord_sdk::voice::DownloadResult::Error(e) => {
            log::error!("[Voice] Model download failed: {e}");

            // Finish global loading indicator
            if let Some(mut loading) =
              self.world.get_resource_mut::<GlobalLoading>()
            {
              loading.finish(LoadingTask::Network);
            }

            if let Some(mut model_state) =
              self.world.get_resource_mut::<VoiceModelState>()
            {
              model_state.set_error(&e);
            }

            self.world.write_message(ToastCommand::error(format!(
              "Voice model download failed: {e}"
            )));

            self.voice_model_download_rx = None;
            break;
          }
        }
      }
    }

    // ========================================================================
    // Poll Compilation Events (from server)
    // ========================================================================

    // Collect events first to avoid borrow conflict.
    let compilation_events: Vec<_> = self
      .compilation_event_rx
      .as_mut()
      .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
      .unwrap_or_default();

    for event in compilation_events {
      self.handle_compilation_event(event);
    }

    // ========================================================================
    // Open SQLite Database (requires runtime.block_on, can't be in ECS system)
    // ========================================================================
    // Note: Result polling, query dispatch, and connection closing are handled
    // by ECS systems in codelord-core::previews::sqlite
    #[cfg(not(target_arch = "wasm32"))]
    {
      // Check if we need to open a new database
      let need_open = self
        .world
        .get_resource::<SqlitePreviewState>()
        .filter(|s| s.enabled && s.needs_reload && !s.is_loading)
        .and_then(|s| s.current_file.clone())
        .filter(|_| {
          // Only open if not already connected
          self
            .world
            .get_resource::<SqliteConnection>()
            .map(|c| !c.is_connected())
            .unwrap_or(true)
        });

      if let Some(file) = need_open {
        let path_str = file.to_string_lossy().to_string();
        log::info!("[SQLite] Opening database: {path_str}");

        // Set loading state
        if let Some(mut state) =
          self.world.get_resource_mut::<SqlitePreviewState>()
        {
          state.is_loading = true;
        }

        // Open database (blocking call - needs runtime)
        let runtime_handle = self.runtime.handle().clone();
        match self.runtime.block_on(codelord_sdk::sqlite::open_database(
          &path_str,
          &runtime_handle,
        )) {
          Ok((query_tx, result_rx)) => {
            // Send initial LoadTables query
            let _ = query_tx.send(SqliteQuery::LoadTables);

            // Store channels in SqliteConnection resource
            if let Some(mut conn) =
              self.world.get_resource_mut::<SqliteConnection>()
            {
              conn.set(query_tx, result_rx);
            }

            log::info!("[SQLite] Database opened, loading tables...");
          }
          Err(e) => {
            log::error!("[SQLite] Failed to open database: {e}");

            if let Some(mut state) =
              self.world.get_resource_mut::<SqlitePreviewState>()
            {
              state.data.error = Some(e);
              state.is_loading = false;
              state.needs_reload = false;
            }
          }
        }
      }
    }

    // ========================================================================
    // Open PDF File (spawns background thread for rendering)
    // ========================================================================
    // Note: Result polling, query dispatch, and connection closing are handled
    // by ECS systems in codelord-core::previews::pdf
    #[cfg(not(target_arch = "wasm32"))]
    {
      // Only load PDF when on Editor page
      let on_editor_page = self
        .world
        .get_resource::<PageResource>()
        .map(|p| p.active_page == page::components::Page::Editor)
        .unwrap_or(false);

      // Check if we need to open a new PDF
      let need_open = on_editor_page
        .then(|| {
          self
            .world
            .get_resource::<PdfPreviewState>()
            .filter(|s| s.enabled && s.is_loading)
            .and_then(|s| s.current_file.clone())
            .filter(|_| {
              // Only open if not already connected
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
            // Store channels in PdfConnection resource
            if let Some(mut conn) =
              self.world.get_resource_mut::<PdfConnection>()
            {
              conn.set(query_tx, result_rx);
            }

            // Note: GlobalLoading and ActiveAnimations are managed by ECS
            // systems (open_pdf_preview_system,
            // update_pdf_preview_on_tab_change)

            log::info!("[PDF] File opened, worker started");
          }
          Err(e) => {
            log::error!("[PDF] Failed to open file: {e}");

            if let Some(mut state) =
              self.world.get_resource_mut::<PdfPreviewState>()
            {
              state.set_error(e);
            }
          }
        }
      }
    }

    // ========================================================================
    // Handle CompileRequest (trigger SDK compilation)
    // ========================================================================

    let compile_requests: Vec<_> = self
      .world
      .query_filtered::<(bevy_ecs::entity::Entity, &CompileRequest), ()>()
      .iter(&self.world)
      .map(|(e, req)| (e, req.source.clone(), req.target.clone(), req.stage))
      .collect();

    for (entity, source, target, stage) in compile_requests {
      log::info!(
        "[Compilation] Triggering compilation for source ({} bytes, stage {})",
        source.len(),
        stage
      );

      // Set compiling state.
      if let Some(mut output) =
        self.world.get_resource_mut::<PlaygroundOutput>()
      {
        output.compilation.is_compiling = true;
        // Clear previous results.
        output.compilation.tokens = None;
        output.compilation.tree = None;
        output.compilation.sir = None;
        output.compilation.asm = None;
        output.compilation.ui = None;
      }

      // Trigger compilation via SDK.
      self.sdk.compile(source, target, stage);

      // Despawn the request entity.
      self.world.despawn(entity);
    }

    // ========================================================================
    // Check for Clear Session Request
    // ========================================================================

    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<ClearSessionRequest>>()
      .iter(&self.world)
      .next() {
        self.clear_session_on_save = true;
        self.world.despawn(entity);
        self.reset_to_fresh_state();
        log::info!("[Session] Session cleared and state reset");
      }

    // ========================================================================
    // Handle Window Requests (need egui::Context)
    // ========================================================================

    // CenterWindow
    if let Some(entity) = self
      .world
      .query_filtered::<bevy_ecs::entity::Entity, bevy_ecs::query::With<CenterWindowRequest>>()
      .iter(&self.world)
      .next()
    {
      log::info!("[Window] Centering window on screen");
      self.world.despawn(entity);

      // Get current position, target center position, and current time
      let animation_data = ctx.input(|i| {
        let current_pos = i.viewport().outer_rect.map(|r| r.min)?;
        let monitor_size = i.viewport().monitor_size?;
        let inner_rect = i.viewport().inner_rect?;
        let window_size = inner_rect.size();

        // Calculate center position
        let center_x = (monitor_size.x - window_size.x) / 2.0;
        let center_y = (monitor_size.y - window_size.y) / 2.0;
        let center_pos = egui::pos2(center_x, center_y);

        Some((current_pos, center_pos, i.time))
      });

      if let Some((start_pos, end_pos, current_time)) = animation_data {
        self.center_animation = Some(CenterWindowAnimation {
          start_time: current_time,
          duration: 0.4, // 400ms - fast but smooth
          start_pos,
          end_pos,
        });
        if let Some(mut active) = self.world.get_resource_mut::<ActiveAnimations>()
        {
          active.increment();
        }
      }
    }

    // ShakeWindow
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

    // PositionWindowLeftHalf
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

    // PositionWindowRightHalf
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

    // ========================================================================
    // Codeshow (Presenter) Message Handling
    // ========================================================================
    {
      use codelord_core::codeshow::{
        CodeshowState, NavigateSlide, PendingPresentationDirectory,
        PendingPresentationFile, SlideDirection,
      };

      // Poll pending file dialog (non-blocking)
      let file_result = self
        .world
        .get_resource::<PendingPresentationFile>()
        .and_then(|pending| pending.0.try_recv().ok());

      if let Some(result) = file_result {
        if let Some(path) = result
          && let Some(mut state) =
            self.world.get_resource_mut::<CodeshowState>()
        {
          let path_str = path.display().to_string();
          if let Err(e) = state.load_file(path) {
            log::error!("[Codeshow] Failed to load presentation file: {e}");
          } else {
            log::info!("[Codeshow] Loaded presentation: {path_str}");
          }
        }

        self.world.remove_resource::<PendingPresentationFile>();
      }

      // Poll pending directory dialog (non-blocking)
      let dir_result = self
        .world
        .get_resource::<PendingPresentationDirectory>()
        .and_then(|pending| pending.0.try_recv().ok());

      if let Some(result) = dir_result {
        if let Some(path) = result
          && let Some(mut state) =
            self.world.get_resource_mut::<CodeshowState>()
        {
          let path_str = path.display().to_string();
          if let Err(e) = state.load_directory(path) {
            log::error!("[Codeshow] Failed to load presentation dir: {e}");
          } else {
            log::info!("[Codeshow] Loaded presentation dir: {path_str}");
          }
        }

        self.world.remove_resource::<PendingPresentationDirectory>();
      }

      // Handle NavigateSlide messages
      let nav_messages: Vec<_> = self
        .world
        .query_filtered::<(bevy_ecs::entity::Entity, &NavigateSlide), ()>()
        .iter(&self.world)
        .map(|(e, msg)| (e, msg.direction))
        .collect();

      for (entity, direction) in nav_messages {
        if let Some(mut state) = self.world.get_resource_mut::<CodeshowState>()
        {
          match direction {
            SlideDirection::Next => state.next(),
            SlideDirection::Previous => state.previous(),
            SlideDirection::First => state.first(),
            SlideDirection::Last => state.last(),
          }
        }

        self.world.despawn(entity);
      }

      // Update transition animation
      if let Some(mut state) = self.world.get_resource_mut::<CodeshowState>()
        && state.is_animating()
      {
        state.update_transition(delta);

        // Mark presenter animation as active
        if let Some(mut cont) =
          self.world.get_resource_mut::<ContinuousAnimations>()
        {
          cont.set_presenter_active();
        }
      }
    }

    // ========================================================================
    // Gilrs Remote Control Input (NORWII N76 and similar presenters)
    // ========================================================================
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(gilrs) = self.gilrs.as_mut() {
      while let Some(event) = gilrs.next_event() {
        match event.event {
          gilrs::EventType::ButtonPressed(button, _) => {
            // NORWII N76 typically maps to these buttons:
            // - Next slide: DPadRight, South (A), or East (B)
            // - Previous slide: DPadLeft, West (X), or North (Y)
            // Some remotes also use triggers
            let direction = match button {
              gilrs::Button::DPadRight
              | gilrs::Button::South
              | gilrs::Button::East
              | gilrs::Button::RightTrigger
              | gilrs::Button::RightTrigger2 => Some(SlideDirection::Next),
              gilrs::Button::DPadLeft
              | gilrs::Button::West
              | gilrs::Button::North
              | gilrs::Button::LeftTrigger
              | gilrs::Button::LeftTrigger2 => Some(SlideDirection::Previous),
              gilrs::Button::DPadUp | gilrs::Button::Start => {
                Some(SlideDirection::First)
              }
              gilrs::Button::DPadDown | gilrs::Button::Select => {
                Some(SlideDirection::Last)
              }
              _ => None,
            };

            if let Some(dir) = direction {
              // Only navigate if presentation is loaded
              let is_loaded = self
                .world
                .get_resource::<CodeshowState>()
                .map(|s| s.is_loaded())
                .unwrap_or(false);

              if is_loaded {
                self.world.spawn(NavigateSlide { direction: dir });
                log::debug!("[Gilrs] Button {:?} -> {:?}", button, dir);
              }
            }
          }
          gilrs::EventType::Connected => {
            let gamepad = gilrs.gamepad(event.id);
            log::info!("[Gilrs] Device connected: {}", gamepad.name());
          }
          gilrs::EventType::Disconnected => {
            log::info!("[Gilrs] Device disconnected: {:?}", event.id);
          }
          _ => {}
        }
      }
    }

    // ========================================================================
    // Run ECS Systems (process commands, events, and animations)
    // ========================================================================
    // Must run BEFORE voice sync so VoiceToggleCommand is processed first.
    self.schedule.run(&mut self.world);

    // ========================================================================
    // Voice Manager Integration (after ECS systems process commands)
    // ========================================================================

    // Get current ECS voice state (now reflects any toggle commands)
    let current_state = self
      .world
      .get_resource::<VoiceResource>()
      .map(|v| v.state)
      .unwrap_or(VoiceState::Idle);

    // Sync voice state from ECS to VoiceManager (detect transitions)
    if let Some(voice_manager) = self.voice_manager.as_mut() {
      voice_manager.try_restore_transcriber();

      // Handle state transitions
      if current_state != self.prev_voice_state {
        match (self.prev_voice_state, current_state) {
          (VoiceState::Idle, VoiceState::Listening) => {
            if let Some(e) = voice_manager.start_listening().err() {
              log::error!("[Voice] Failed to start listening: {e}");

              if let Some(mut voice) =
                self.world.get_resource_mut::<VoiceResource>()
              {
                voice.set_error(e.to_string());
              }
            }
          }
          (VoiceState::Listening, VoiceState::Idle) => {
            voice_manager.stop_listening();
          }
          _ => {}
        }
      }

      // Sync waveform data from VoiceManager to ECS resource
      let vm_status = voice_manager.get_status();
      let waveform = voice_manager.get_waveform();

      // Convert codelord_voice status to ECS VisualizerStatus
      let status = match vm_status {
        codelord_voice::VisualizerStatus::Idle => VisualizerStatus::Idle,
        codelord_voice::VisualizerStatus::Listening => {
          VisualizerStatus::Listening
        }
        codelord_voice::VisualizerStatus::Processing => {
          VisualizerStatus::Processing
        }
        codelord_voice::VisualizerStatus::Speaking => {
          VisualizerStatus::Speaking
        }
        codelord_voice::VisualizerStatus::Success => VisualizerStatus::Success,
        codelord_voice::VisualizerStatus::Error => VisualizerStatus::Error,
      };

      if let Some(mut voice_res) =
        self.world.get_resource_mut::<VoiceResource>()
      {
        voice_res.waveform = waveform;
        voice_res.set_visualizer_status(status);

        // Update state from visualizer status (Processing, etc.)
        match vm_status {
          codelord_voice::VisualizerStatus::Processing
            if voice_res.state == VoiceState::Listening =>
          {
            voice_res.set_state(VoiceState::Processing);
          }
          codelord_voice::VisualizerStatus::Success => {
            voice_res.set_state(VoiceState::Idle);
          }
          codelord_voice::VisualizerStatus::Error
            if voice_res.state != VoiceState::Idle =>
          {
            voice_res.set_error("Voice processing failed");
          }
          _ => {}
        }
      }

      // Trigger shake animation on error transition
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

    // Track state for next frame
    self.prev_voice_state = current_state;

    // ========================================================================
    // Update Center Window Animation
    // ========================================================================

    let center_complete = self
      .center_animation
      .as_ref()
      .map(|anim| {
        let current_time = ctx.input(|i| i.time);
        let elapsed = current_time - anim.start_time;
        let progress = (elapsed / anim.duration).min(1.0) as f32;

        if progress < 1.0 {
          // OutExpo easing: 1 - 2^(-10 * progress)
          let eased = 1.0 - 2.0_f32.powf(-10.0 * progress);

          let new_x =
            anim.start_pos.x + (anim.end_pos.x - anim.start_pos.x) * eased;
          let new_y =
            anim.start_pos.y + (anim.end_pos.y - anim.start_pos.y) * eased;

          ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
            egui::pos2(new_x, new_y),
          ));
          false
        } else {
          // Animation complete - set final position
          ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
            anim.end_pos,
          ));
          true
        }
      })
      .unwrap_or(false);

    // Clear center animation and decrement counter if complete
    if center_complete {
      self.center_animation = None;
      if let Some(mut active) =
        self.world.get_resource_mut::<ActiveAnimations>()
      {
        active.decrement();
      }
    }

    // ========================================================================
    // Update Shake Animation
    // ========================================================================

    let shake_complete = self
      .shake_animation
      .as_ref()
      .map(|shake| {
        let current_time = ctx.input(|i| i.time);
        let elapsed = current_time - shake.start_time;
        let progress = (elapsed / shake.duration).min(1.0) as f32;

        if progress < 1.0 {
          let frequency = 20.0;
          let damping = 3.0;
          let wave = (elapsed * frequency * std::f64::consts::TAU).sin() as f32;
          let amplitude = shake.intensity * (1.0 - progress).powf(damping);
          let seed = (elapsed * frequency).floor() as u32;
          let offset_x = wave
            * amplitude
            * (if seed.is_multiple_of(2) { 1.0 } else { -1.0 });
          let offset_y = wave
            * amplitude
            * 0.7
            * (if seed.is_multiple_of(3) { 1.0 } else { -1.0 });

          let new_pos = egui::pos2(
            shake.original_x + offset_x,
            shake.original_y + offset_y,
          );
          ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(new_pos));
          false
        } else {
          let original_pos = egui::pos2(shake.original_x, shake.original_y);
          ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
            original_pos,
          ));
          true
        }
      })
      .unwrap_or(false);

    // Clear shake animation and decrement counter if complete
    if shake_complete {
      self.shake_animation = None;
      if let Some(mut active) =
        self.world.get_resource_mut::<ActiveAnimations>()
      {
        active.decrement();
      }
    }

    // Mark shake animation as active in ECS (for ContinuousAnimations)
    if self.shake_animation.is_some()
      && let Some(mut anim) =
        self.world.get_resource_mut::<ContinuousAnimations>()
    {
      anim.set_shake_active();
    }

    // Mark voice animation as active if not idle (waveform, progress bar, etc.)
    let voice_animating = self
      .world
      .get_resource::<VoiceResource>()
      .map(|v| !matches!(v.visualizer_status, VisualizerStatus::Idle))
      .unwrap_or(false);

    voice_animating.then(|| {
      self
        .world
        .get_resource_mut::<ContinuousAnimations>()
        .map(|mut anim| anim.set_voice_active())
    });

    // ========================================================================
    // Apply Theme (animated or static)
    // ========================================================================
    let visuals = assets::theme::get_animated_visuals(&self.world);
    ctx.set_visuals(visuals);

    // ========================================================================
    // HTML Preview WebView Integration (after UI rendered, rect available)
    // ========================================================================
    #[cfg(not(target_arch = "wasm32"))]
    {
      // Set window handle on first frame
      if !self.html_preview_handle_set
        && let Ok(window_handle) = frame.window_handle()
      {
        self
          .html_preview_webview
          .set_window_handle(window_handle.as_raw());

        self.html_preview_handle_set = true;

        log::debug!("[HtmlPreview] Window handle set");
      }

      // Read preview state from ECS
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

      // Sync visibility
      if enabled && !self.html_preview_webview.visible {
        self.html_preview_webview.show();
        self.html_preview_webview.try_create_webview();

        log::debug!("[HtmlPreview] WebView shown");
      } else if !enabled && self.html_preview_webview.visible {
        self.html_preview_webview.hide();
        log::debug!("[HtmlPreview] WebView hidden");
      }

      // Handle reload request (file changed)
      if enabled && needs_reload {
        if let Some(file_path) = &current_file {
          // Send file path to SDK server for preview
          let path_str = file_path.to_string_lossy().to_string();

          self.sdk.send_html_preview_file(path_str);
          self.html_preview_webview.reload();

          log::debug!("[HtmlPreview] Updated preview file: {file_path:?}");
        }

        // Clear the reload flag
        if let Some(mut s) = self.world.get_resource_mut::<HtmlPreviewState>() {
          s.needs_reload = false;
        }
      }

      // Update bounds if visible and rect available
      if enabled && let Some(r) = rect {
        self
          .html_preview_webview
          .update_bounds(r.x, r.y, r.width, r.height);
      }
    }

    // ========================================================================
    // Playground WebView Integration (for templating mode)
    // ========================================================================
    #[cfg(not(target_arch = "wasm32"))]
    {
      // Set window handle on first frame
      if !self.playground_handle_set
        && let Ok(window_handle) = frame.window_handle()
      {
        self
          .playground_webview
          .set_window_handle(window_handle.as_raw());

        self.playground_handle_set = true;

        log::debug!("[PlaygroundPreview] Window handle set");
      }

      // Read playground webview state from ECS
      let (enabled, rect, needs_reload) = self
        .world
        .get_resource::<PlaygroundWebviewState>()
        .map(|s| (s.enabled, s.webview_rect, s.needs_reload))
        .unwrap_or((false, None, false));

      // Sync visibility
      if enabled && !self.playground_webview.visible {
        self.playground_webview.show();
        self.playground_webview.try_create_webview();
      } else if !enabled && self.playground_webview.visible {
        self.playground_webview.hide();
      }

      // Handle reload request (compilation updated)
      if enabled && needs_reload {
        self.playground_webview.reload();
        log::debug!("[PlaygroundPreview] WebView reloaded");

        // Clear the reload flag
        if let Some(mut s) =
          self.world.get_resource_mut::<PlaygroundWebviewState>()
        {
          s.needs_reload = false;
        }
      }

      // Update bounds if visible and rect available
      if enabled && let Some(r) = rect {
        self
          .playground_webview
          .update_bounds(r.x, r.y, r.width, r.height);
      }
    }

    // ========================================================================
    // Process continuous animations (wave, stripe, cursor blink)
    // ========================================================================
    let animation_changes = self
      .world
      .get_resource_mut::<ContinuousAnimations>()
      .map(|mut cont| cont.end_frame());

    if let Some((increments, decrements)) = animation_changes
      && (increments > 0 || decrements > 0)
      && let Some(mut active) =
        self.world.get_resource_mut::<ActiveAnimations>()
    {
      (0..increments).for_each(|_| active.increment());
      (0..decrements).for_each(|_| active.decrement());
    }

    // ========================================================================
    // Request repaint if any animations are active
    // ========================================================================

    if self
      .world
      .get_resource::<ActiveAnimations>()
      .filter(|a| a.has_active())
      .is_some()
    {
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

    // Draw app border (on top of everything)
    let content_rect = ctx.input(|i| i.viewport_rect());
    let border_color = egui::Color32::from_gray(30);

    ctx
      .layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("app_border"),
      ))
      .rect_stroke(
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
          .fill(ctx.global_style().visuals.window_fill)
          // .inner_margin(egui::Margin::symmetric(8, 0)),
      )
      .show_separator_line(false)
      .show_inside(ui, |ui| {
        titlebar::show(ui, &mut self.world);
        self.render_header_separator(ui);
      });

    // Search panel (rendered at top level with animation)
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
          search_panel::show(ui, &mut self.world);
        });

      // Signal search hint animation at top level (since show_animated
      // doesn't call show() when panel is hidden)
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
      .show_inside(ui, |ui| statusbar::show(ui, &mut self.world));

    // Music player panel (above statusbar).
    // Get animated height for playlist expansion.
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
        let snapshot = audio::get_music_snapshot();

        // Calculate progress based on playback position.
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

        // Only make progress bar interactive when there's a track loaded.
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

          // Handle click to seek.
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

              audio::music_seek(seek_position);
            }
          }

          // Change cursor on hover.
          if progress_response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
          }
        }

        // Progress bar background track.
        ui.painter().line_segment(
          [
            egui::pos2(rect.left(), separator_y),
            egui::pos2(rect.left() + total_width, separator_y),
          ],
          egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 60, 60)),
        );

        // Progress bar current position (lime green).
        let progress_width = total_width * progress_ratio;
        ui.painter().line_segment(
          [
            egui::pos2(rect.left(), separator_y),
            egui::pos2(rect.left() + progress_width, separator_y),
          ],
          egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(204, 253, 62)),
        );

        // Sync UI state with actual playback state from audio thread.
        if let Some(ref snap) = snapshot
          && let Some(mut state) =
            self.world.get_resource_mut::<MusicPlayerState>()
        {
          state.is_playing = snap.state == audio::PlaybackState::Playing;
        }

        music_player::show(ui, &mut self.world);
      });

    // Read animated zoom margin for central panel (pure data, no method calls)
    let zoom_margin = self
      .world
      .get_resource::<tabbar::ZoomState>()
      .map(|z| {
        z.transition
          .as_ref()
          .map(|t| t.animated_margin)
          .unwrap_or(if z.is_zoomed { 4.0 } else { 0.0 })
      })
      .unwrap_or(0.0);

    let margin_i8 = zoom_margin.round() as i8;
    let central_frame = egui::Frame::NONE
      .fill(if zoom_margin > 0.0 {
        egui::Color32::WHITE
      } else {
        ctx.global_style().visuals.window_fill
      })
      .inner_margin(egui::Margin::same(margin_i8));

    egui::CentralPanel::default()
      .frame(central_frame)
      .show_inside(ui, |ui| base::show(ui, &mut self.world));

    overlays::popup::show(&ctx, &mut self.world);

    // Render filescope overlay
    let filescope_response = overlays::filescope::show(&ctx, &mut self.world);

    self.handle_filescope_response(filescope_response);

    // Render unsaved changes dialog
    let unsaved_response =
      overlays::unsaved_changes_dialog::show(&ctx, &mut self.world);

    self.handle_unsaved_changes_response(unsaved_response);

    // Render toast notifications overlay
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

    // Magic zoom: apply transform to every overlay layer (popups, file
    // picker, dialogs, toasts) now that they've all rendered.
    self.propagate_magic_zoom(&ctx);

    // Check if voice model download toast should be shown
    self.check_voice_model_toast();

    // Handle keyboard shortcuts
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::T)) {
      self.world.write_message(ThemeCommand {
        action: ThemeAction::Toggle,
      });
    }

    // Tab navigation: Cmd+Shift+[ for previous, Cmd+Shift+] for next
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

    // Voice control: Cmd+Shift+Space
    if ctx.input(|i| {
      i.modifiers.command
        && i.modifiers.shift
        && i.key_pressed(egui::Key::Space)
    }) {
      self.world.write_message(VoiceToggleCommand);
    }

    // Save file: Cmd+S
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
      use codelord_core::ecs::query::With;

      // Find active editor tab and spawn save request.
      let active_editor = self
        .world
        .query_filtered::<codelord_core::ecs::entity::Entity, (With<EditorTab>, With<Active>)>()
        .iter(&self.world)
        .next();

      if let Some(entity) = active_editor {
        self.world.spawn(SaveFileRequest::new(entity));
      }
    }

    // Search: Cmd+F
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
      self.world.spawn(ToggleSearchRequest);
    }

    // Git blame: Cmd+Shift+G
    if ctx.input(|i| {
      i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::G)
    }) {
      self.world.spawn(ToggleBlameRequest);
    }

    // Filescope: Cmd+P (Quick Open)
    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P)) {
      self
        .world
        .resource_mut::<FilescopeState>()
        .toggle(FilescopeMode::Files);
    }

    // Music player: Cmd+M
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

    // Magic zoom: hold Cmd+E (Screen-Studio-style).
    //
    // Held-key, not toggle: matches the codelord "held-key modality"
    // doctrine (like Cmd+Shift+Space for voice). Emit the command only on
    // transition to avoid spamming Messages every frame; retarget the
    // camera center each frame while held so the zoom follows the cursor.
    //
    // Hotkey is hardcoded; user-configurable binding deferred to a later
    // PR (tracked alongside the broader keybinds UI work).
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

  /// Render header separator with optional voice progress bar.
  fn render_header_separator(&mut self, ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let separator_y = rect.bottom() - 1.0;

    // Codelord colors
    const GREEN_100: egui::Color32 = egui::Color32::from_rgb(204, 253, 62);
    const GREEN_200: egui::Color32 = egui::Color32::from_rgb(6, 208, 1);
    const RED_100: egui::Color32 = egui::Color32::from_rgb(221, 3, 3);

    // Base separator line (always visible)
    ui.painter().line_segment(
      [
        egui::pos2(rect.left(), separator_y),
        egui::pos2(rect.right(), separator_y),
      ],
      egui::Stroke::new(1.0_f32, egui::Color32::from_gray(30)),
    );

    // Get voice status from ECS resource (pure ECS, no Arc<Mutex>)
    let (status, processing_start_time) = self
      .world
      .get_resource::<VoiceResource>()
      .map(|v| (v.visualizer_status, v.processing_start_time))
      .unwrap_or((VisualizerStatus::Idle, 0));

    // Check global loading state
    let (is_global_loading, is_global_completed, loading_start_time) = self
      .world
      .get_resource::<GlobalLoading>()
      .map(|l| (l.is_loading(), l.is_completed(), l.start_time))
      .unwrap_or((false, false, 0));

    match status {
      VisualizerStatus::Processing => {
        let now = std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_millis() as u64;

        let elapsed_ms = now.saturating_sub(processing_start_time);
        let elapsed_secs = elapsed_ms as f32 / 1000.0;

        let k = 0.5;
        let progress = (1.0 - (-k * elapsed_secs).exp()).min(0.95);
        let progress_width = rect.width() * progress;

        let progress_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), separator_y),
          egui::vec2(progress_width, 2.0),
        );

        ui.painter().rect_filled(progress_rect, 0.0, GREEN_100);
      }
      VisualizerStatus::Success => {
        let progress_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), separator_y),
          egui::vec2(rect.width(), 2.0),
        );

        ui.painter().rect_filled(progress_rect, 0.0, GREEN_200);
      }
      VisualizerStatus::Error => {
        let progress_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), separator_y),
          egui::vec2(rect.width(), 2.0),
        );

        ui.painter().rect_filled(progress_rect, 0.0, RED_100);
      }
      _ if is_global_completed => {
        // Show full bar when completed (100%)
        let progress_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), separator_y),
          egui::vec2(rect.width(), 2.0),
        );

        ui.painter().rect_filled(progress_rect, 0.0, GREEN_200);

        if let Some(mut anims) =
          self.world.get_resource_mut::<ContinuousAnimations>()
        {
          anims.set_loading_bar_active();
        }
      }
      _ if is_global_loading => {
        let now = std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_millis() as u64;

        let elapsed_ms = now.saturating_sub(loading_start_time);
        let elapsed_secs = elapsed_ms as f32 / 1000.0;

        let k = 0.5;
        let progress = (1.0 - (-k * elapsed_secs).exp()).min(0.95);
        let progress_width = rect.width() * progress;

        let progress_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), separator_y),
          egui::vec2(progress_width, 2.0),
        );

        ui.painter().rect_filled(progress_rect, 0.0, GREEN_100);

        if let Some(mut anims) =
          self.world.get_resource_mut::<ContinuousAnimations>()
        {
          anims.set_loading_bar_active();
        }
      }
      _ => {}
    }
  }

  /// Handle response from filescope.
  fn handle_filescope_response(&mut self, response: FilescopeResponse) {
    match response {
      FilescopeResponse::Select(path, _action) => {
        self.world.spawn(OpenFileRequest::new(path));
        self.world.resource_mut::<FilescopeState>().hide();
      }
      FilescopeResponse::Close => {
        self.world.resource_mut::<FilescopeState>().hide();
      }
      FilescopeResponse::None => {}
    }
  }

  /// Handle response from unsaved changes dialog.
  fn handle_unsaved_changes_response(
    &mut self,
    response: UnsavedChangesResponse,
  ) {
    match response {
      UnsavedChangesResponse::None => {
        // Dialog still open, do nothing
      }
      UnsavedChangesResponse::Save => {
        // Save and close the tab
        let entity = self.world.resource::<UnsavedChangesDialog>().entity;

        if let Some(entity) = entity {
          // Spawn save request
          self.world.spawn(SaveFileRequest::new(entity));
          // Note: Don't close tab here - save system will handle it
          // and we may need to wait for "Save As" dialog for new tabs
        }

        self.world.resource_mut::<UnsavedChangesDialog>().close();
      }
      UnsavedChangesResponse::DontSave => {
        // Close tab without saving
        let entity = self.world.resource::<UnsavedChangesDialog>().entity;

        if let Some(entity) = entity {
          // Get tab info for activating next tab
          let tab_order =
            self.world.get::<Tab>(entity).map(|t| t.order).unwrap_or(0);

          // Find and activate next tab
          let next_entity: Option<bevy_ecs::entity::Entity> = self
            .world
            .query_filtered::<(bevy_ecs::entity::Entity, &Tab), bevy_ecs::prelude::With<codelord_core::tabbar::EditorTab>>()
            .iter(&self.world)
            .filter(|(e, _)| *e != entity)
            .min_by_key(|(_, t)| {
              if t.order > tab_order {
                t.order
              } else {
                u32::MAX - t.order
              }
            })
            .map(|(e, _)| e);

          // Deactivate current, activate next
          self.world.entity_mut(entity).remove::<Active>();
          if let Some(next) = next_entity {
            self.world.entity_mut(next).insert(Active);
          }

          // Despawn the tab
          self.world.despawn(entity);
        }

        self.world.resource_mut::<UnsavedChangesDialog>().close();
      }
      UnsavedChangesResponse::Cancel => {
        // Just close the dialog, don't close the tab
        self.world.resource_mut::<UnsavedChangesDialog>().close();
      }
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

        // Start global loading indicator
        if let Some(mut loading) =
          self.world.get_resource_mut::<GlobalLoading>()
        {
          loading.start(LoadingTask::Network);
        }

        // Spawn download in background
        let download_rx = codelord_sdk::voice::spawn_download();
        self.voice_model_download_rx = Some(download_rx);
      }
      _ => {
        log::debug!("[Toast] Unknown action: {action_id}");
      }
    }
  }

  /// Check if voice model download toast should be shown.
  fn check_voice_model_toast(&mut self) {
    let should_show = self
      .world
      .get_resource::<VoiceModelState>()
      .map(|s| s.show_download_toast)
      .unwrap_or(false);

    if should_show {
      // Clear the flag immediately to avoid duplicate toasts
      if let Some(mut model_state) =
        self.world.get_resource_mut::<VoiceModelState>()
      {
        model_state.dismiss_toast();
      }

      // Send toast command with action buttons
      self.world.write_message(
        ToastCommand::info("Voice model required (~148 MB)").with_actions(
          vec![ToastAction::new("voice_download", "Download").stripe()],
        ),
      );
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

    log::info!(
      "[Session] Restoring: {} tabs, theme: {}, roots: {}",
      session.tabs.len(),
      session.theme.kind,
      session.explorer.roots.len()
    );

    // Restore theme
    if let Some(mut theme_res) = world.get_resource_mut::<ThemeResource>() {
      theme_res.current = match session.theme.kind.as_str() {
        "light" => ThemeKind::Light,
        "custom" => ThemeKind::Custom,
        _ => ThemeKind::Dark,
      }
    }

    // Restore panel visibility
    if let Some(mut left) = world.get_resource_mut::<LeftPanelResource>() {
      left.is_visible = session.panels.left_visible;
    }

    if let Some(mut right) = world.get_resource_mut::<RightPanelResource>() {
      right.is_visible = session.panels.right_visible;
    }

    if let Some(mut bottom) = world.get_resource_mut::<BottomPanelResource>() {
      bottom.is_visible = session.panels.bottom_visible;
    }

    // Restore explorer roots (triggers directory scan via systems)
    if !session.explorer.roots.is_empty() {
      if let Some(mut explorer) = world.get_resource_mut::<ExplorerState>() {
        explorer.roots = session.explorer.roots.clone();
      }

      // Set active workspace to first root
      if let Some(first_root) = session.explorer.roots.first()
        && let Some(mut active_ws) =
          world.get_resource_mut::<ActiveWorkspaceRoot>()
      {
        active_ws.path = Some(first_root.clone());
        active_ws.name = first_root
          .file_name()
          .map(|n| n.to_string_lossy().to_string());
      }
    }

    // Restore tabs
    if session.tabs.is_empty() {
      return false;
    }

    session
      .tabs
      .iter()
      .enumerate()
      .for_each(|(idx, tab_state)| {
        let order = world
          .get_resource_mut::<TabOrderCounter>()
          .map(|mut counter| counter.next())
          .unwrap_or(idx as u32);

        // Determine content: reload from disk if file exists and not dirty,
        // otherwise use saved content (for unsaved changes or new files).
        let content = if !tab_state.is_dirty {
          tab_state
            .path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_else(|| tab_state.content.clone())
        } else {
          tab_state.content.clone()
        };

        let mut buffer = TextBuffer::new(&content);
        buffer.modified = tab_state.is_dirty;

        let mut entity = world.spawn((
          Tab::new(&tab_state.title, order),
          EditorTab,
          SonarAnimation::default(),
          buffer,
          Cursor::new(tab_state.cursor_position.min(content.len())),
          codelord_core::symbol::TabSymbols::new(),
          codelord_core::git::components::TabBlame::new(),
          Focusable,
          KeyboardHandler::text_editor(),
        ));

        // Add FileTab if path exists
        tab_state
          .path
          .as_ref()
          .map(|path| entity.insert(FileTab::new(path.clone())));

        // Mark as active if this was the active tab
        (session.active_tab_index == Some(idx)).then(|| entity.insert(Active));
      });

    // Auto-enable preview if active tab is a binary file (SQLite, PDF, or XLS)
    // Spawn ECS requests to handle preview activation
    if let Some(active_idx) = session.active_tab_index
      && let Some(tab_state) = session.tabs.get(active_idx)
      && let Some(path) = &tab_state.path
    {
      if Self::is_sqlite_file(path) {
        if let Some(mut sqlite_preview) =
          world.get_resource_mut::<SqlitePreviewState>()
        {
          sqlite_preview.enabled = true;
          sqlite_preview.current_file = Some(path.clone());
          sqlite_preview.needs_reload = true;
        }
      } else if Self::is_pdf_file(path) {
        world.spawn(OpenPdfPreviewRequest(path.clone()));
      } else if Self::is_xls_file(path) {
        if let Some(mut xls_preview) =
          world.get_resource_mut::<XlsPreviewState>()
        {
          xls_preview.open(path.clone());
        }
      } else if Self::is_font_file(path) {
        if let Some(mut font_preview) =
          world.get_resource_mut::<FontPreviewState>()
        {
          font_preview.open(path);
        }
      } else if Self::is_svg_file(path)
        && let Some(mut svg_preview) =
          world.get_resource_mut::<SvgPreviewState>()
      {
        svg_preview.open(path);
      }
    }

    log::info!(
      "[Session] Restored {} tabs, active: {:?}",
      session.tabs.len(),
      session.active_tab_index
    );

    true
  }

  /// Checks if a file is a SQLite database by extension.
  fn is_sqlite_file(path: &std::path::Path) -> bool {
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| {
        matches!(ext.to_lowercase().as_str(), "db" | "sqlite" | "sqlite3")
      })
      .unwrap_or(false)
  }

  /// Checks if a file is a PDF by extension.
  fn is_pdf_file(path: &std::path::Path) -> bool {
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext.eq_ignore_ascii_case("pdf"))
      .unwrap_or(false)
  }

  /// Checks if a file is an Excel spreadsheet by extension.
  fn is_xls_file(path: &std::path::Path) -> bool {
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| {
        matches!(
          ext.to_lowercase().as_str(),
          "xls" | "xlsx" | "xlsm" | "xlsb" | "ods"
        )
      })
      .unwrap_or(false)
  }

  /// Checks if a file is a font file by extension.
  fn is_font_file(path: &std::path::Path) -> bool {
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| {
        matches!(
          ext.to_lowercase().as_str(),
          "ttf" | "otf" | "woff" | "woff2"
        )
      })
      .unwrap_or(false)
  }

  /// Checks if a file is an SVG by extension.
  fn is_svg_file(path: &std::path::Path) -> bool {
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext.eq_ignore_ascii_case("svg"))
      .unwrap_or(false)
  }

  /// Reset the app to a fresh state (clear all tabs, explorer, etc.)
  fn reset_to_fresh_state(&mut self) {
    use codelord_core::ecs::query::With;
    use codelord_core::navigation::components::FileEntry;

    // Collect all editor tab entities to despawn
    let editor_tabs: Vec<codelord_core::ecs::entity::Entity> = self
      .world
      .query_filtered::<codelord_core::ecs::entity::Entity, With<EditorTab>>()
      .iter(&self.world)
      .collect();

    // Collect all file entries (explorer items) to despawn
    let file_entries: Vec<codelord_core::ecs::entity::Entity> = self
      .world
      .query_filtered::<codelord_core::ecs::entity::Entity, With<FileEntry>>()
      .iter(&self.world)
      .collect();

    // Despawn all editor tabs
    editor_tabs.iter().for_each(|&entity| {
      self.world.despawn(entity);
    });

    // Despawn all file entries
    file_entries.iter().for_each(|&entity| {
      self.world.despawn(entity);
    });

    // Clear explorer roots
    if let Some(mut explorer) = self.world.get_resource_mut::<ExplorerState>() {
      explorer.roots.clear();
    }

    // Reset panel visibility to defaults
    if let Some(mut left) = self.world.get_resource_mut::<LeftPanelResource>() {
      left.is_visible = true;
    }

    if let Some(mut right) = self.world.get_resource_mut::<RightPanelResource>()
    {
      right.is_visible = false;
    }

    if let Some(mut bottom) =
      self.world.get_resource_mut::<BottomPanelResource>()
    {
      bottom.is_visible = false;
    }

    // Reset tab order counter
    if let Some(mut counter) = self.world.get_resource_mut::<TabOrderCounter>()
    {
      counter.reset();
    }

    // Create a fresh playground tab
    let order = self
      .world
      .get_resource_mut::<TabOrderCounter>()
      .map(|mut counter| counter.next())
      .unwrap_or(0);

    self.world.spawn((
      Tab::new("playground-1", order),
      PlaygroundTab,
      SonarAnimation::default(),
      TextBuffer::empty(),
      Cursor::new(0),
      Active,
      Focusable,
      KeyboardHandler::text_editor(),
    ));
  }

  /// Handle compilation events from the server.
  fn handle_compilation_event(&mut self, event: CompilationEvent) {
    use codelord_protocol::compilation::Stage;

    match event {
      CompilationEvent::Started => {
        log::info!("[Compilation] Started");

        if let Some(mut output) =
          self.world.get_resource_mut::<PlaygroundOutput>()
        {
          output.compilation.is_compiling = true;
        }
        if let Some(mut feedback) =
          self.world.get_resource_mut::<PlaygroundFeedback>()
        {
          feedback.state = FeedbackState::Running;
        }
      }
      CompilationEvent::Stage {
        stage,
        data,
        elapsed_time,
      } => {
        log::info!(
          "[Compilation] Stage {stage:?} complete ({} bytes, {elapsed_time:.3}time)",
          data.len(),
        );

        if let Some(mut output) =
          self.world.get_resource_mut::<PlaygroundOutput>()
        {
          output.compilation.elapsed_time = elapsed_time;
          match stage {
            Stage::Tokens => {
              output.compilation.token_count =
                data.matches("\"kind\":").count() - 1; // -1 for EOF.
              output.compilation.tokens = Some(data);
            }
            Stage::Tree => {
              output.compilation.node_count =
                data.matches("\"token\":").count();
              output.compilation.tree = Some(data);
            }
            Stage::Sir => {
              output.compilation.insn_count = data.matches("\"kind\":").count();
              output.compilation.sir = Some(data);
            }
            Stage::Asm => {
              output.compilation.asm_bytes = data.len();
              output.compilation.asm = Some(data);
            }
            Stage::Ui => {
              // Count commands in the JSON array
              output.compilation.ui_count =
                data.matches("\"BeginContainer\"").count()
                  + data.matches("\"EndContainer\"").count()
                  + data.matches("\"Text\"").count()
                  + data.matches("\"Button\"").count()
                  + data.matches("\"TextInput\"").count()
                  + data.matches("\"Image\"").count();
              output.compilation.ui = Some(data);

              // Trigger webview reload when UI stage completes
              if let Some(mut webview_state) =
                self.world.get_resource_mut::<PlaygroundWebviewState>()
              {
                webview_state.needs_reload = true;
              }
            }
          }
        }
      }
      CompilationEvent::Error { message, span } => {
        log::warn!("[Compilation] Error: {message} at {span:?}");

        if let Some(mut output) =
          self.world.get_resource_mut::<PlaygroundOutput>()
        {
          output.compilation.is_compiling = false;
        }
      }
      CompilationEvent::Done { success } => {
        log::info!("[Compilation] Done (success: {success})");

        if let Some(mut output) =
          self.world.get_resource_mut::<PlaygroundOutput>()
        {
          output.compilation.is_compiling = false;
        }
        if let Some(mut feedback) =
          self.world.get_resource_mut::<PlaygroundFeedback>()
        {
          feedback.state = if success {
            FeedbackState::Success
          } else {
            FeedbackState::Ready
          };
        }
      }
    }
  }
}

pub mod radius {
  use eframe::egui;

  pub fn symmetric(north: u8, south: u8) -> egui::CornerRadius {
    egui::CornerRadius {
      nw: north,
      ne: north,
      sw: south,
      se: south,
    }
  }
}
