//! Magic zoom systems.

use super::messages::MagicZoomCommand;
use super::resources::MagicZoomState;
use crate::animation::components::DeltaTime;

use bevy_ecs::message::MessageReader;
use bevy_ecs::system::{Res, ResMut};

/// Drains `MagicZoomCommand` messages to retarget the zoom, then advances
/// all eased scalars by frame dt.
pub fn update_magic_zoom_system(
  mut state: ResMut<MagicZoomState>,
  mut commands: MessageReader<MagicZoomCommand>,
  dt: Res<DeltaTime>,
) {
  for cmd in commands.read() {
    state.retarget_zoom(cmd.engage);
  }

  state.tick(dt.delta());
}

#[cfg(test)]
mod tests {
  use super::*;

  use bevy_ecs::message::Messages;
  use bevy_ecs::schedule::Schedule;
  use bevy_ecs::world::World;

  /// Minimal world with only the resources this system reads.
  fn make_world(dt_per_frame: f32) -> (World, Schedule) {
    let mut world = World::new();

    world.insert_resource(MagicZoomState::default());
    world.insert_resource({
      let mut dt = DeltaTime::default();
      dt.update(dt_per_frame);
      dt
    });
    world.init_resource::<Messages<MagicZoomCommand>>();

    let mut schedule = Schedule::default();

    schedule.add_systems(update_magic_zoom_system);

    (world, schedule)
  }

  #[test]
  fn system_engages_on_command() {
    let (mut world, mut schedule) = make_world(0.016);

    world.write_message(MagicZoomCommand { engage: true });

    schedule.run(&mut world);

    let state = world.resource::<MagicZoomState>();

    assert!(state.engaged);
    assert!(state.is_animating());
  }

  #[test]
  fn system_advances_toward_target_over_frames() {
    let (mut world, mut schedule) = make_world(0.016);

    world.write_message(MagicZoomCommand { engage: true });

    // One full second of 60fps frames — well past the zoom duration.
    for _ in 0..60 {
      schedule.run(&mut world);
    }

    let zoom = world.resource::<MagicZoomState>().zoom();

    assert!((zoom - 2.0).abs() < 0.001, "zoom = {zoom}");
  }

  #[test]
  fn system_disengage_returns_to_idle() {
    let (mut world, mut schedule) = make_world(0.016);

    world.write_message(MagicZoomCommand { engage: true });

    for _ in 0..60 {
      schedule.run(&mut world);
    }

    world.write_message(MagicZoomCommand { engage: false });

    for _ in 0..60 {
      schedule.run(&mut world);
    }

    let state = world.resource::<MagicZoomState>();
    assert!(!state.engaged);
    assert!((state.zoom() - 1.0).abs() < 0.001);
  }

  #[test]
  fn system_without_commands_still_ticks() {
    // No command emitted — the system must still advance any in-flight
    // animation (e.g. zoom settling after a previous engage).
    let (mut world, mut schedule) = make_world(0.1);
    world.resource_mut::<MagicZoomState>().retarget_zoom(true);

    schedule.run(&mut world);

    let state = world.resource::<MagicZoomState>();
    assert!(state.zoom() > 1.0, "zoom should have advanced");
  }
}
