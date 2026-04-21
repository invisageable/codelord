pub mod resources;
pub mod systems;

pub use resources::{
  CompilationState, FeedbackState, OutputViewKind, PLAYGROUND_PREVIEW_URL,
  PlaygroundFeedback, PlaygroundHoveredSpan, PlaygroundMetrics,
  PlaygroundOutput, PlaygroundWebviewState, TemplatingTarget,
};
pub use systems::{activate_playground_tab_system, new_playground_tab_system};

/// Insert playground resources and spawn the output/time metric entities.
///
/// Spawning the metric entities here (and storing their ids in
/// `PlaygroundMetrics`) keeps playground setup in one place. Also
/// inserts [`crate::navigation::resources::StagebarResource`] with the
/// compiler-stages preset — the stagebar is a playground concern.
pub fn install(world: &mut crate::ecs::world::World) {
  use crate::navigation::resources::StagebarResource;
  use crate::playground::resources::{
    PlaygroundFeedback, PlaygroundHoveredSpan, PlaygroundMetrics,
    PlaygroundOutput, PlaygroundWebviewState,
  };
  use crate::ui::component::Metric;

  let output_metric = world
    .spawn(Metric::new(
      "OUTPUT",
      "Total size of the compiled output in bytes.",
      0.0,
      "tokens",
      [255, 255, 255, 255],
    ))
    .id();

  let time_metric = world
    .spawn(Metric::new_time(
      "TIME",
      "Total compilation time in milliseconds.",
      0.0,
      [204, 255, 0, 255],
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
  world.insert_resource(StagebarResource::compiler_stages());
}

/// Spawn the default first-run playground tab with a zo hello-world
/// snippet. Called only when session restore didn't bring anything back.
pub fn spawn_default_tab(world: &mut crate::ecs::world::World) {
  use crate::keyboard::{Focusable, KeyboardHandler};
  use crate::tabbar::components::{PlaygroundTab, SonarAnimation, Tab};
  use crate::tabbar::resources::TabOrderCounter;
  use crate::text_editor::components::{Cursor, FileTab, TextBuffer};
  use crate::ui::component::Active;

  const DEFAULT_CONTENT: &str = r#"fun main() {
  imu view: </> ::= <>hello world!</>;
  #dom view;
}
"#;

  let order = world
    .get_resource_mut::<TabOrderCounter>()
    .map(|mut counter| counter.next())
    .unwrap_or(0);

  world.spawn((
    Tab::new("playground-1", order),
    PlaygroundTab,
    SonarAnimation::default(),
    TextBuffer::new(DEFAULT_CONTENT),
    Cursor::new(0),
    FileTab::new("playground-1.zo"),
    Active,
    Focusable,
    KeyboardHandler::text_editor(),
  ));
}

/// Register playground systems.
pub fn register_systems(schedule: &mut crate::ecs::schedule::Schedule) {
  schedule
    .add_systems((new_playground_tab_system, activate_playground_tab_system));
}

/// Apply a streamed [`codelord_protocol::compilation::CompilationEvent`]
/// to the playground resources (output, feedback, webview reload flag).
///
/// Called by the app shell for each event arriving on the SDK compilation
/// channel. Pure resource mutation — no egui, no tokio.
pub fn apply_compilation_event(
  world: &mut crate::ecs::world::World,
  event: codelord_protocol::compilation::CompilationEvent,
) {
  use crate::playground::resources::{
    FeedbackState, PlaygroundFeedback, PlaygroundOutput, PlaygroundWebviewState,
  };
  use codelord_protocol::compilation::{CompilationEvent, Stage};

  match event {
    CompilationEvent::Started => {
      log::info!("[Compilation] Started");

      if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>() {
        output.compilation.is_compiling = true;
      }
      if let Some(mut feedback) = world.get_resource_mut::<PlaygroundFeedback>()
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

      if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>() {
        output.compilation.elapsed_time = elapsed_time;

        match stage {
          Stage::Tokens => {
            output.compilation.token_count =
              data.matches("\"kind\":").count() - 1; // -1 for EOF.
            output.compilation.tokens = Some(data);
          }
          Stage::Tree => {
            output.compilation.node_count = data.matches("\"token\":").count();
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
            output.compilation.ui_count =
              data.matches("\"BeginContainer\"").count()
                + data.matches("\"EndContainer\"").count()
                + data.matches("\"Text\"").count()
                + data.matches("\"Button\"").count()
                + data.matches("\"TextInput\"").count()
                + data.matches("\"Image\"").count();
            output.compilation.ui = Some(data);
          }
        }
      }

      if matches!(stage, Stage::Ui)
        && let Some(mut webview_state) =
          world.get_resource_mut::<PlaygroundWebviewState>()
      {
        webview_state.needs_reload = true;
      }
    }
    CompilationEvent::Error { message, span } => {
      log::warn!("[Compilation] Error: {message} at {span:?}");

      if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>() {
        output.compilation.is_compiling = false;
      }
    }
    CompilationEvent::Done { success } => {
      log::info!("[Compilation] Done (success: {success})");

      if let Some(mut output) = world.get_resource_mut::<PlaygroundOutput>() {
        output.compilation.is_compiling = false;
      }
      if let Some(mut feedback) = world.get_resource_mut::<PlaygroundFeedback>()
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
