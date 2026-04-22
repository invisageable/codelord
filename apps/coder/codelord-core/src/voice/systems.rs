//! Voice control ECS systems.

use crate::animation::components::DeltaTime;
use crate::audio::resources::{AudioDispatcher, MusicPlayerState, Playlist};
use crate::ecs::message::Messages;
use crate::ecs::world::World;
use crate::events::{
  CenterWindowRequest, CloseAllTabsRequest, CloseOtherTabsRequest,
  CloseTabRequest, CloseTabsToRightRequest, CompileRequest,
  NewEditorTabRequest, OpenFolderDialogRequest, PositionWindowLeftHalfRequest,
  PositionWindowRightHalfRequest, SaveFileRequest, ShakeWindowRequest,
};
use crate::navigation::resources::{PlaygroundMode, StagebarResource};
use crate::page::components::Page;
use crate::page::resources::PageSwitchCommand;
use crate::panel::resources::{PanelAction, PanelCommand};
use crate::playground::{PlaygroundOutput, TemplatingTarget};
use crate::tabbar::components::EditorTab;
use crate::tabbar::components::PlaygroundTab;
use crate::text_editor::components::TextBuffer;
use crate::ui::component::Active;
use crate::voice::components::VoiceState;
use crate::voice::resources::{
  ModelStatus, VoiceActionEvent, VoiceModelState, VoiceResource,
  VoiceToggleCommand,
};

use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageReader;
use bevy_ecs::query::With;
use bevy_ecs::system::{Res, ResMut};

/// System: Process voice toggle commands.
///
/// Toggles between Idle and Listening states.
/// If model is not ready, prompts for download instead.
pub fn voice_toggle_system(
  mut commands: MessageReader<VoiceToggleCommand>,
  mut voice: ResMut<VoiceResource>,
  mut model_state: ResMut<VoiceModelState>,
) {
  for _ in commands.read() {
    log::info!("[Voice] Toggle received, status={:?}", model_state.status);

    // Check model status before allowing voice control
    match model_state.status {
      ModelStatus::Missing | ModelStatus::Unknown => {
        // Prompt user to download the model
        model_state.prompt_download();
        log::info!(
          "[Voice] Model missing, show_toast={}",
          model_state.show_download_toast
        );

        continue;
      }
      ModelStatus::Downloading => {
        // Already downloading, ignore toggle
        log::debug!("[Voice] Toggle ignored - model downloading");

        continue;
      }
      ModelStatus::Error => {
        // Had an error, prompt again
        model_state.prompt_download();
        log::warn!("[Voice] Model error, prompting download");

        continue;
      }
      ModelStatus::Ready => {
        // Model ready, proceed with toggle
      }
    }

    match voice.state {
      VoiceState::Idle => {
        voice.set_state(VoiceState::Listening);
        log::info!("[Voice] Started listening");
      }
      VoiceState::Listening => {
        // Stop listening and return to idle
        voice.set_state(VoiceState::Idle);
        log::info!("[Voice] Stopped listening");
      }
      _ => {
        // Can't toggle while processing/executing
        log::debug!("[Voice] Toggle ignored - currently {:?}", voice.state);
      }
    }
  }
}

/// System: Update voice animation state.
pub fn voice_animation_system(
  dt: Res<DeltaTime>,
  mut voice: ResMut<VoiceResource>,
) {
  voice.update_animation(dt.delta());
}

/// System: Process voice action events and execute them.
///
/// Exclusive system that converts voice action strings to actual ECS commands.
pub fn voice_action_system(world: &mut World) {
  // Read voice action events
  let events: Vec<VoiceActionEvent> = world
    .get_resource_mut::<Messages<VoiceActionEvent>>()
    .map(|mut msgs| msgs.drain().collect())
    .unwrap_or_default();

  for event in events {
    log::info!("[Voice] Executing action: {}", event.action);

    // Update voice resource
    if let Some(mut voice) = world.get_resource_mut::<VoiceResource>() {
      voice.last_action = Some(event.action.clone());

      voice.set_state(VoiceState::Idle);
    }

    // Execute action based on name and optional payload
    // Handles both server format (PascalCase) and parser format (snake_case)
    execute_voice_action(world, &event.action, event.payload.as_deref());
  }
}

/// Execute voice action - same format as codelord Action::from_voice_command.
fn execute_voice_action(
  world: &mut World,
  action: &str,
  payload: Option<&str>,
) {
  match action {
    "NewTab" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world.spawn(NewEditorTabRequest);
    }
    "CloseActiveTab" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world
        .query_filtered::<Entity, (With<EditorTab>, With<Active>)>()
        .iter(world)
        .next()
        .map(|entity| world.spawn(CloseTabRequest::new(entity)));
    }
    "CloseAllTabs" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world.spawn(CloseAllTabsRequest);
    }
    "CloseOtherTabs" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world
        .query_filtered::<Entity, (With<EditorTab>, With<Active>)>()
        .iter(world)
        .next()
        .map(|entity| world.spawn(CloseOtherTabsRequest::new(entity)));
    }
    "CloseTabsToRight" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world
        .query_filtered::<Entity, (With<EditorTab>, With<Active>)>()
        .iter(world)
        .next()
        .map(|entity| world.spawn(CloseTabsToRightRequest::new(entity)));
    }
    "OpenFile" => {
      log::info!("[Voice] Open file dialog not yet implemented");
    }
    "OpenFolder" => {
      world.spawn(OpenFolderDialogRequest);
    }
    "SaveActiveTab" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world
        .query_filtered::<Entity, (With<EditorTab>, With<Active>)>()
        .iter(world)
        .next()
        .map(|entity| world.spawn(SaveFileRequest::new(entity)));
    }
    "ToggleExplorer" => {
      world.write_message(PanelCommand {
        action: PanelAction::ToggleLeft,
      });
    }
    "ToggleTerminal" => {
      world.write_message(PageSwitchCommand { page: Page::Editor });
      world.write_message(PanelCommand {
        action: PanelAction::ToggleBottom,
      });
    }
    "ToggleCopilord" => {
      world.write_message(PanelCommand {
        action: PanelAction::ToggleRight,
      });
    }
    "ToggleSearch" => {
      log::info!("[Voice] Toggle search not yet implemented");
    }
    "CenterWindow" => {
      world.spawn(CenterWindowRequest);
    }
    "ShakeWindow" => {
      world.spawn(ShakeWindowRequest);
    }
    "PositionWindowLeftHalf" => {
      world.spawn(PositionWindowLeftHalfRequest);
    }
    "PositionWindowRightHalf" => {
      world.spawn(PositionWindowRightHalfRequest);
    }
    "SwitchToPage" => {
      payload
        .and_then(|page_name| match page_name {
          "Welcome" => Some(Page::Welcome),
          "Editor" => Some(Page::Editor),
          "Playground" => Some(Page::Playground),
          "Notes" => Some(Page::Notes),
          "Settings" => Some(Page::Settings),
          _ => None,
        })
        .map(|page| world.write_message(PageSwitchCommand { page }));
    }
    "RunTokensStage" => {
      run_compiler_stage(world, 0, PlaygroundMode::Programming);
    }
    "RunTreeStage" => {
      run_compiler_stage(world, 1, PlaygroundMode::Programming);
    }
    "RunSirStage" => {
      run_compiler_stage(world, 2, PlaygroundMode::Programming);
    }
    "RunAsmStage" => {
      run_compiler_stage(world, 3, PlaygroundMode::Programming);
    }
    "RunUiStage" => {
      run_compiler_stage(world, 3, PlaygroundMode::Templating);
    }
    "PlayPauseMusic" => {
      let audio = world
        .get_resource::<AudioDispatcher>()
        .copied()
        .unwrap_or_default();

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
    "TogglePlayer" => {
      let current_time = world
        .get_resource::<DeltaTime>()
        .map(|dt| dt.elapsed())
        .unwrap_or(0.0);

      if let Some(mut state) = world.get_resource_mut::<MusicPlayerState>() {
        state.toggle_visibility(current_time);
      }
    }
    _ => log::warn!("[Voice] Unknown action: {action}"),
  }
}

/// Run a compiler stage: switch to playground, select stage, and compile.
fn run_compiler_stage(world: &mut World, stage: usize, mode: PlaygroundMode) {
  world.write_message(PageSwitchCommand {
    page: Page::Playground,
  });

  if let Some(mut stagebar) = world.get_resource_mut::<StagebarResource>() {
    stagebar.set_mode(mode);
    stagebar.select(stage);
  }

  // Get current templating target.
  let target = world
    .get_resource::<PlaygroundOutput>()
    .map(|o| o.templating_target)
    .unwrap_or_default();

  let target_str = match target {
    TemplatingTarget::Web => "web",
    TemplatingTarget::Native => "native",
  };

  // Get source from active playground tab and compile.
  let source = world
    .query_filtered::<&TextBuffer, (With<PlaygroundTab>, With<Active>)>()
    .iter(world)
    .next()
    .map(|buffer| buffer.rope.to_string())
    .unwrap_or_default();

  if !source.is_empty() {
    use codelord_protocol::compilation::Stage;

    let protocol_stage = match (stage, mode) {
      (0, _) => Stage::Tokens,
      (1, _) => Stage::Tree,
      (2, _) => Stage::Sir,
      (3, PlaygroundMode::Programming) => Stage::Asm,
      (3, PlaygroundMode::Templating) => Stage::Ui,
      _ => Stage::Tokens,
    };

    world.spawn(CompileRequest::new(source, target_str, protocol_stage));
  }
}
