//! Settings page ECS resources.

use crate::animation::hacker::HackerAnimation;
use crate::ecs::prelude::*;
use crate::settings::system_info::SystemInfo;

use codelord_i18n::t;

/// Navigation mode for settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsNavMode {
  #[default]
  Categories,
  Items,
}

/// Setting item types.
#[derive(Debug, Clone)]
pub enum SettingItem {
  Toggle {
    label: &'static str,
    description: &'static str,
    value: bool,
  },
  Selector {
    label: &'static str,
    description: &'static str,
    options: Vec<&'static str>,
    selected: usize,
  },
  Text {
    label: &'static str,
    description: &'static str,
    value: String,
  },
  Action {
    label: &'static str,
    description: &'static str,
    action_label: &'static str,
  },
}

impl SettingItem {
  pub fn label(&self) -> &str {
    match self {
      Self::Toggle { label, .. } => label,
      Self::Selector { label, .. } => label,
      Self::Text { label, .. } => label,
      Self::Action { label, .. } => label,
    }
  }

  pub fn description(&self) -> &str {
    match self {
      Self::Toggle { description, .. } => description,
      Self::Selector { description, .. } => description,
      Self::Text { description, .. } => description,
      Self::Action { description, .. } => description,
    }
  }
}

/// Settings category.
#[derive(Debug, Clone)]
pub struct SettingsCategory {
  /// Translation key for the category name.
  pub name: &'static str,
  pub items: Vec<SettingItem>,
}

impl SettingsCategory {
  /// Get the translated category name.
  pub fn name(&self) -> String {
    t!(self.name).to_string()
  }
}

/// ECS Resource for the Settings page state.
#[derive(Resource, Debug, Clone)]
pub struct SettingsResource {
  /// All categories.
  pub categories: Vec<SettingsCategory>,
  /// Current navigation mode.
  pub nav_mode: SettingsNavMode,
  /// Focused category index (when in Categories mode).
  pub focused_category: usize,
  /// Selected category index (when in Items mode).
  pub selected_category: usize,
  /// Focused item index (when in Items mode).
  pub focused_item: usize,
  /// Focus bar current Y position.
  pub focus_bar_y: f32,
  /// Focus bar target Y position.
  pub focus_bar_target_y: f32,
  /// Item Y positions for focus bar.
  pub item_y_positions: Vec<f32>,
  /// Hacker animation for category name in header.
  pub category_name_animation: Option<HackerAnimation>,
  /// Hacker animation for item description in right column.
  pub item_description_animation: Option<HackerAnimation>,
}

impl Default for SettingsResource {
  fn default() -> Self {
    let sys_info = SystemInfo::new();

    Self {
      categories: vec![
        SettingsCategory {
          name: "settings.categories.app",
          items: vec![
            SettingItem::Selector {
              label: "Language",
              description: "Set the language of the entire app.",
              options: vec![
                "English — English",
                "Français — French",
                "中文 — Chinese",
                "日本語 — Japanese",
              ],
              selected: 0,
            },
            SettingItem::Toggle {
              label: "Enable Sounds",
              description: "Play sound effects for UI interactions.",
              value: true,
            },
            SettingItem::Toggle {
              label: "Auto Save",
              description: "Automatically save files when switching tabs.",
              value: false,
            },
            SettingItem::Toggle {
              label: "Restore Session",
              description: "Restore open files when starting the app.",
              value: true,
            },
            SettingItem::Action {
              label: "Clear Session",
              description: "Delete saved session data. Next restart will start fresh.",
              action_label: "Clear",
            },
            SettingItem::Action {
              label: "Microphone Permission",
              description: "Open System Settings to grant microphone access for voice control.",
              action_label: "Open",
            },
          ],
        },
        SettingsCategory {
          name: "settings.categories.appearance",
          items: vec![
            SettingItem::Selector {
              label: "Theme",
              description: "Select the color theme for the interface.",
              options: vec!["KURO", "SHiVA"],
              selected: 0,
            },
            SettingItem::Toggle {
              label: "Cursor Animation",
              description: "Enable animated cursor effects in the editor.",
              value: true,
            },
            SettingItem::Selector {
              label: "Font Size",
              description: "Adjust the editor font size.",
              options: vec!["12", "14", "16", "18", "20"],
              selected: 1,
            },
          ],
        },
        SettingsCategory {
          name: "settings.categories.editor",
          items: vec![
            SettingItem::Toggle {
              label: "Git Blame",
              description: "Show git blame annotation at end of current line. Toggle with Cmd+Shift+G.",
              value: true,
            },
            SettingItem::Selector {
              label: "Blame Min Column",
              description: "Minimum column position for inline blame text.",
              options: vec!["40", "60", "80", "100"],
              selected: 1,
            },
          ],
        },
        SettingsCategory {
          name: "settings.categories.system",
          items: vec![
            SettingItem::Text {
              label: "Operating System",
              description: "Current operating system and version.",
              value: sys_info.os_display(),
            },
            SettingItem::Text {
              label: "Kernel Version",
              description: "Operating system kernel version.",
              value: sys_info.kernel_version.clone(),
            },
            SettingItem::Text {
              label: "CPU",
              description: "Processor model and core count.",
              value: sys_info.cpu_display(),
            },
            SettingItem::Text {
              label: "Total Memory",
              description: "Total system RAM installed.",
              value: sys_info.total_memory_display(),
            },
            SettingItem::Text {
              label: "Available Memory",
              description: "Currently available system RAM.",
              value: sys_info.available_memory_display(),
            },
            SettingItem::Text {
              label: "App Version",
              description: "Current application version.",
              value: sys_info.app_version.clone(),
            },
            SettingItem::Text {
              label: "Process ID",
              description: "Current process identifier.",
              value: sys_info.process_id.to_string(),
            },
            SettingItem::Action {
              label: "Copy System Info",
              description: "Copy all system information to clipboard for bug reports.",
              action_label: "Copy",
            },
          ],
        },
      ],
      nav_mode: SettingsNavMode::Categories,
      focused_category: 0,
      selected_category: 0,
      focused_item: 0,
      // Will be set to absolute screen Y on first render
      focus_bar_y: 0.0,
      focus_bar_target_y: 0.0,
      item_y_positions: Vec::new(),
      category_name_animation: None,
      item_description_animation: None,
    }
  }
}

impl SettingsResource {
  /// Navigate up.
  pub fn navigate_up(&mut self) {
    match self.nav_mode {
      SettingsNavMode::Categories => {
        if self.focused_category > 0 {
          self.focused_category -= 1;
        }
      }
      SettingsNavMode::Items => {
        if self.focused_item > 0 {
          self.focused_item -= 1;
          // Start hacker animation for new item description
          if let Some(item) = self.focused_item_data() {
            self.item_description_animation =
              Some(HackerAnimation::new(item.description()));
          }
        }
      }
    }
  }

  /// Navigate down.
  pub fn navigate_down(&mut self) {
    match self.nav_mode {
      SettingsNavMode::Categories => {
        if self.focused_category < self.categories.len().saturating_sub(1) {
          self.focused_category += 1;
        }
      }
      SettingsNavMode::Items => {
        let item_count = self
          .categories
          .get(self.selected_category)
          .map(|c| c.items.len())
          .unwrap_or(0);
        if self.focused_item < item_count.saturating_sub(1) {
          self.focused_item += 1;
          // Start hacker animation for new item description
          if let Some(item) = self.focused_item_data() {
            self.item_description_animation =
              Some(HackerAnimation::new(item.description()));
          }
        }
      }
    }
  }

  /// Enter the focused category.
  pub fn enter_category(&mut self) {
    if self.nav_mode == SettingsNavMode::Categories {
      self.selected_category = self.focused_category;
      self.focused_item = 0;
      self.nav_mode = SettingsNavMode::Items;

      // Start hacker animation for category name
      if let Some(name) = self.selected_category_name() {
        self.category_name_animation = Some(HackerAnimation::new(name));
      }

      // Start hacker animation for first item description
      if let Some(item) = self.focused_item_data() {
        self.item_description_animation =
          Some(HackerAnimation::new(item.description()));
      }
    }
  }

  /// Go back to categories view.
  pub fn back_to_categories(&mut self) {
    if self.nav_mode == SettingsNavMode::Items {
      self.nav_mode = SettingsNavMode::Categories;
      self.category_name_animation = None;
      self.item_description_animation = None;
    }
  }

  /// Activate the focused item (toggle, cycle selector).
  /// Returns true if it's an Action item that needs external handling.
  pub fn activate_item(&mut self) -> bool {
    if self.nav_mode != SettingsNavMode::Items {
      return false;
    }

    if let Some(category) = self.categories.get_mut(self.selected_category)
      && let Some(item) = category.items.get_mut(self.focused_item)
    {
      match item {
        SettingItem::Toggle { value, .. } => {
          *value = !*value;
        }
        SettingItem::Selector {
          options, selected, ..
        } => {
          *selected = (*selected + 1) % options.len();
        }
        SettingItem::Text { .. } => {
          // Text items are read-only
        }
        SettingItem::Action { .. } => {
          // Return true to signal external handling needed
          return true;
        }
      }
    }
    false
  }

  /// Check if the focused item is the "Copy System Info" action.
  pub fn is_copy_system_info_action(&self) -> bool {
    self
      .focused_item_data()
      .map(|item| {
        matches!(item, SettingItem::Action { label, .. } if *label == "Copy System Info")
      })
      .unwrap_or(false)
  }

  /// Check if the focused item is the "Clear Session" action.
  pub fn is_clear_session_action(&self) -> bool {
    self
      .focused_item_data()
      .map(|item| {
        matches!(item, SettingItem::Action { label, .. } if *label == "Clear Session")
      })
      .unwrap_or(false)
  }

  /// Check if the focused item is the "Microphone Permission" action.
  pub fn is_microphone_permission_action(&self) -> bool {
    self
      .focused_item_data()
      .map(|item| {
        matches!(item, SettingItem::Action { label, .. } if *label == "Microphone Permission")
      })
      .unwrap_or(false)
  }

  /// Get system info formatted for clipboard.
  pub fn get_system_info_text(&self) -> String {
    SystemInfo::new().format()
  }

  /// Cycle selector left.
  pub fn selector_left(&mut self) {
    if self.nav_mode != SettingsNavMode::Items {
      return;
    }

    if let Some(category) = self.categories.get_mut(self.selected_category)
      && let Some(SettingItem::Selector { selected, .. }) =
        category.items.get_mut(self.focused_item)
      && *selected > 0
    {
      *selected -= 1;
    }
  }

  /// Cycle selector right.
  pub fn selector_right(&mut self) {
    if self.nav_mode != SettingsNavMode::Items {
      return;
    }

    if let Some(category) = self.categories.get_mut(self.selected_category)
      && let Some(SettingItem::Selector {
        options, selected, ..
      }) = category.items.get_mut(self.focused_item)
      && *selected < options.len().saturating_sub(1)
    {
      *selected += 1;
    }
  }

  /// Update focus bar position target (uses absolute screen Y).
  pub fn update_focus_bar_target(&mut self) {
    let index = match self.nav_mode {
      SettingsNavMode::Categories => self.focused_category,
      SettingsNavMode::Items => self.focused_item,
    };

    if let Some(&y) = self.item_y_positions.get(index) {
      self.focus_bar_target_y = y;
    }
  }

  /// Update focus bar animation.
  pub fn update_focus_bar(&mut self, dt: f32) -> bool {
    let speed = 8.0;
    let delta = (self.focus_bar_target_y - self.focus_bar_y) * speed * dt;
    self.focus_bar_y += delta;
    delta.abs() > 0.01
  }

  /// Get the focused category name.
  pub fn selected_category_name(&self) -> Option<String> {
    self
      .categories
      .get(self.selected_category)
      .map(|c| c.name())
  }

  /// Get items for the selected category.
  pub fn selected_items(&self) -> Option<&[SettingItem]> {
    self
      .categories
      .get(self.selected_category)
      .map(|c| c.items.as_slice())
  }

  /// Get the focused item.
  pub fn focused_item_data(&self) -> Option<&SettingItem> {
    self
      .categories
      .get(self.selected_category)
      .and_then(|c| c.items.get(self.focused_item))
  }
}
