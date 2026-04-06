use crate::icon::components::Icon;
use crate::page::components::Page;

use std::path::PathBuf;

/// A category displayed on the X-axis (horizontal).
#[derive(Debug, Clone)]
pub struct XmbCategory {
  /// The name of the category.
  pub name: String,
  /// The icon of the category.
  pub icon: Icon,
  /// The collection of items.
  pub items: Vec<XmbItem>,
  /// The color of the category (RGBA).
  pub color: [u8; 4],
}

/// An item displayed on the Y-axis (vertical).
#[derive(Debug, Clone)]
pub struct XmbItem {
  /// The name of the item.
  pub name: String,
  /// The description of the item.
  pub description: String,
  /// The icon of the item.
  pub icon: Icon,
  /// The action to perform when this item is selected.
  pub action: XmbAction,
}

/// Actions that can be triggered from XMB items.
#[derive(Debug, Clone)]
pub enum XmbAction {
  NewFile,
  OpenFile,
  OpenFolder,
  OpenRecentFile(PathBuf),
  SwitchToPage(Page),
  OpenSettings,
  OpenThemeSelector,
  Exit,
}

/// Navigation directions for XMB interface.
#[derive(Debug, Clone, Copy)]
pub enum XmbNavigation {
  Left,
  Right,
  Up,
  Down,
  Select,
  /// Jump to a specific category (for click events).
  JumpToCategory(usize),
  /// Jump to a specific item (for click events).
  JumpToItem(usize),
}
