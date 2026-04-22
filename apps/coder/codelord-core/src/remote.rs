//! Remote control / gamepad input (gilrs).
//!
//! Polls a [`gilrs::Gilrs`] instance once per frame, translates known
//! presenter-remote buttons (NORWII N76 and similar) into
//! [`crate::codeshow::NavigateSlide`] spawns when a presentation is
//! loaded. The Gilrs handle itself is held as a non-Send resource
//! because its internal OS handles aren't `Send`.

use crate::codeshow::{CodeshowState, NavigateSlide, SlideDirection};
use crate::ecs::schedule::Schedule;
use crate::ecs::world::World;

/// Non-Send wrapper around [`gilrs::Gilrs`]. `None` when gilrs failed
/// to initialize (no gamepad subsystem on this host, missing perms,
/// etc.) — the poll system short-circuits in that case.
pub struct RemoteResource(pub Option<gilrs::Gilrs>);

/// Initialize gilrs and insert it as a non-Send resource.
pub fn install(world: &mut World) {
  let gilrs = gilrs::Gilrs::new()
    .map_err(|err| log::warn!("[Remote] Failed to initialize gilrs: {err}"))
    .ok();

  world.insert_non_send_resource(RemoteResource(gilrs));
}

/// Register the per-frame poll as an exclusive system.
pub fn register_systems(schedule: &mut Schedule) {
  schedule.add_systems(poll_remote);
}

/// Drain gilrs events and translate them into
/// [`NavigateSlide`] spawns when a presentation is loaded.
pub fn poll_remote(world: &mut World) {
  let Some(mut remote) = world.get_non_send_resource_mut::<RemoteResource>()
  else {
    return;
  };

  let Some(gilrs) = remote.0.as_mut() else {
    return;
  };

  // Drain gilrs events first — we can't read other resources while
  // `remote` holds a mutable borrow of the world.
  let mut spawns: Vec<SlideDirection> = Vec::new();

  while let Some(event) = gilrs.next_event() {
    match event.event {
      gilrs::EventType::ButtonPressed(button, _) => {
        // NORWII N76 and similar presenter remotes typically map to:
        // - Next slide: DPadRight, South (A), East (B), right triggers
        // - Previous slide: DPadLeft, West (X), North (Y), left triggers
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
          spawns.push(dir);
          log::debug!("[Remote] Button {button:?} -> {dir:?}");
        }
      }
      gilrs::EventType::Connected => {
        let gamepad = gilrs.gamepad(event.id);
        log::info!("[Remote] Device connected: {}", gamepad.name());
      }
      gilrs::EventType::Disconnected => {
        log::info!("[Remote] Device disconnected: {:?}", event.id);
      }
      _ => {}
    }
  }

  drop(remote);

  if spawns.is_empty() {
    return;
  }

  let is_loaded = world
    .get_resource::<CodeshowState>()
    .map(|s| s.is_loaded())
    .unwrap_or(false);

  if !is_loaded {
    return;
  }

  for direction in spawns {
    world.spawn(NavigateSlide { direction });
  }
}
