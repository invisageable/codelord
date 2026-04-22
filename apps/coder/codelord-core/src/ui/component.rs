//! UI interaction components

pub mod active;
pub mod clickable;
pub mod counter;
pub mod decoration;
pub mod focused;
pub mod hovered;
pub mod metric;
pub mod modified;

// Re-export commonly used items
pub use active::Active;
pub use clickable::{Clickable, Clicked};
pub use counter::Counter;
pub use decoration::{DecorationBundle, DecorationType};
pub use focused::Focused;
pub use hovered::Hovered;
pub use metric::{Metric, MetricUnit};
pub use modified::Modified;
