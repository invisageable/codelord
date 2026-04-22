//! Titlebar initial entity spawning.
//!
//! Owns the default window chrome (close / minimize / fullscreen) and the
//! default titlebar navigation icons that ship with the codelord shell.

use crate::ecs::world::World;

/// Spawn the default titlebar entities: window decorations (close /
/// minimize+maximize / fullscreen) and nav icons (home / code / ufo /
/// alien) with their drag order.
pub fn spawn_default(world: &mut World) {
  use crate::drag_and_drop::DragOrder;
  use crate::icon::components::{Icon, TitlebarIconBundle};
  use crate::ui::component::{DecorationBundle, DecorationType};

  world.spawn(DecorationBundle::new(DecorationType::Close));
  world.spawn(DecorationBundle::new(DecorationType::MinimizeMaximize));
  world.spawn(DecorationBundle::new(DecorationType::Fullscreen));

  world.spawn((TitlebarIconBundle::new(Icon::Home), DragOrder(0)));
  world.spawn((TitlebarIconBundle::new(Icon::Code), DragOrder(1)));
  world.spawn((TitlebarIconBundle::new(Icon::Ufo), DragOrder(2)));
  world.spawn((TitlebarIconBundle::new(Icon::Alien), DragOrder(3)));
}
