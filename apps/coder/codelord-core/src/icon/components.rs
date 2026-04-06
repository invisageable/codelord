use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;

/// Icon component for icon button entities
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Icon {
  Add,
  Alien,
  Arrow(Arrow),
  Binary,
  Browser,
  Byakugan(Byakugan),
  Close,
  Code,
  Collapse,
  Copilord,
  Dot(Dot),
  Download(Download),
  Explorer,
  Feedback(Feedback),
  Folder(Folder),
  Hacker,
  Home,
  Key(Key),
  Keyboard,
  Language(Language),
  Layout(Layout),
  Notes,
  Player(Player),
  Preview(Preview),
  Quote,
  Refresh,
  Schema,
  Search,
  Server,
  Sound,
  Structure(Structure),
  Table,
  Terminal,
  Theme,
  Ufo,
  Voice,
  Zoom(Zoom),
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Arrow {
  Left,
  Right,
  AngleLeftLine,
  AngleRightLine,
  DoubleRight,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Byakugan {
  On,
  Off,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Dot {
  Horizontal,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Download {
  Cloud,
  Folder,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Feedback {
  Alert,
  Info,
  Success,
  SuccessRounded,
  Warning,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Folder {
  Open,
  Close,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Key {
  Alt,
  Backspace,
  Cmd,
  Enter,
  Shift,
  Space,
  Tab,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Layout {
  Custom,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Preview {
  Markdown,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Structure {
  File,
  FolderOpen,
  FolderClose,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Language {
  Bash,
  C,
  Clojure,
  Cplusplus,
  Css,
  Csv,
  Dart,
  Database,
  Docker,
  Elixir,
  Env,
  Erlang,
  Excel,
  Favicon,
  Font,
  GitIgnore,
  Github,
  Gleam,
  Go,
  Html,
  Image,
  JavaScript,
  Json,
  Kotlin,
  License,
  Lock,
  Makefile,
  Markdown,
  Music,
  Nim,
  Ocaml,
  Pdf,
  Python,
  React,
  Ruby,
  Rust,
  Sass,
  Sqlite,
  Svg,
  Svelte,
  Toml,
  TypeScript,
  Video,
  Vite,
  Vitest,
  Vue,
  Wasm,
  Wat,
  Yaml,
  Zig,
  Zip,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
  MusicNote,
  Muted,
  Next,
  Pause,
  Play,
  Playlist,
  Replay,
  Stop,
  Volume,
}

#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Zoom {
  InArrow,
  OutArrow,
}

/// Marker: icon belongs to titlebar
#[derive(Component, Default)]
pub struct TitlebarIcon;

/// Marker: icon belongs to statusbar
#[derive(Component, Default)]
pub struct StatusbarIcon;

/// Bundle for spawning generic icon button entities
#[derive(Bundle)]
pub struct IconBundle {
  pub icon: Icon,
  pub hovered: super::super::ui::component::hovered::Hovered,
  pub focused: super::super::ui::component::focused::Focused,
  pub clickable: super::super::ui::component::clickable::Clickable,
}

impl IconBundle {
  pub fn new(icon: Icon) -> Self {
    Self {
      icon,
      hovered: super::super::ui::component::hovered::Hovered,
      focused: super::super::ui::component::focused::Focused,
      clickable: super::super::ui::component::clickable::Clickable::default(),
    }
  }
}

/// Bundle for spawning titlebar icon buttons
#[derive(Bundle)]
pub struct TitlebarIconBundle {
  pub icon: Icon,
  pub marker: TitlebarIcon,
  pub hovered: super::super::ui::component::hovered::Hovered,
  pub focused: super::super::ui::component::focused::Focused,
  pub clickable: super::super::ui::component::clickable::Clickable,
}

impl TitlebarIconBundle {
  pub fn new(icon: Icon) -> Self {
    Self {
      icon,
      marker: TitlebarIcon,
      hovered: super::super::ui::component::hovered::Hovered,
      focused: super::super::ui::component::focused::Focused,
      clickable: super::super::ui::component::clickable::Clickable::default(),
    }
  }
}

/// Bundle for spawning statusbar icon buttons
#[derive(Bundle)]
pub struct StatusbarIconBundle {
  pub icon: Icon,
  pub marker: StatusbarIcon,
  pub hovered: super::super::ui::component::hovered::Hovered,
  pub focused: super::super::ui::component::focused::Focused,
  pub clickable: super::super::ui::component::clickable::Clickable,
}

impl StatusbarIconBundle {
  pub fn new(icon: Icon) -> Self {
    Self {
      icon,
      marker: StatusbarIcon,
      hovered: super::super::ui::component::hovered::Hovered,
      focused: super::super::ui::component::focused::Focused,
      clickable: super::super::ui::component::clickable::Clickable::default(),
    }
  }
}
