//! About page ECS resources.

use crate::animation::hacker::HackerAnimation;
use crate::ecs::resource::Resource;

use swisskit::core::fmt::zo;

/// The intro text (first paragraph).
const INTRO_TEXT: &str = r#"Based in all corners of the world, we are members of Compilords. A small group of technology enthusiasts whose sole goal is to push their limits to the extreme. Inspired by video games, we want every software machine to have the tools it needs to boost its productivity.

Our mastery of cutting-edge contemporary technologies, combined with a refined aesthetic sense, gives us the flexibility to bring innovative concepts to life. We believe in independence and independent people.

We are alive software crafters."#;

/// The full manifesto text.
const MANIFESTO_TEXT: &str = r#"The conclusion is simple: we regret the days when intelligent people and artists worked hand in hand to design user interfaces that were ahead of their time. However, profit has killed this kind of collaboration. We find ourselves trapped in a software prison, where every application looks the same. Time is no longer allocated to quality; it is used for marketing to fill the human brain with banalities. Providing emotions is no longer the key; let's face it, the mainstream doesn't care. Linus Torvalds was right...

Fortunately, we still have the freedom to take matters into our own hands. We'll do the job ourselves. They can all go to hell, each to their own quaalude. The time has come to bring innovation back into fashion. We'll take on the mantle of Dreamcast and try to learn from their mistakes. Because if we continue down this path, the entire internet will become our own prison. Given that we're not waiting for anyone to save us, we've vowed to do everything we can to use all our creativity to innovate on a large scale in order to inspire anyone who believes in our ideas.

Those who want to stick with the old-fashioned design, stick with it. However, those who see the passion and attention to detail in our proposal, this is only the beginning. Our expertise is sharp, we're fired up. The future isn't going to stop anytime soon. We're going to offer the entire community software they won't soon forget.

Join the devolution"#;

/// ECS Resource for the About page state.
#[derive(Resource, Debug, Clone)]
pub struct AboutResource {
  /// The intro hacker animation state.
  pub intro_animation: HackerAnimation,
  /// Whether the intro text was visible in the previous frame.
  pub intro_was_visible: bool,
  /// The manifesto hacker animation state.
  pub manifesto_animation: HackerAnimation,
  /// Whether the manifesto text was visible in the previous frame.
  pub manifesto_was_visible: bool,
}

impl Default for AboutResource {
  fn default() -> Self {
    Self {
      intro_animation: HackerAnimation::new(zo::format(INTRO_TEXT)),
      intro_was_visible: false,
      manifesto_animation: HackerAnimation::new(zo::format(MANIFESTO_TEXT)),
      manifesto_was_visible: false,
    }
  }
}
