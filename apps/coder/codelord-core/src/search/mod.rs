//! Search functionality for the IDE.
//!
//! This module provides search state and resources using pure ECS principles.

pub mod engine;
pub mod resources;

pub use engine::{perform_search, perform_search_str, validate_regex};
pub use resources::{SearchOption, SearchState};
