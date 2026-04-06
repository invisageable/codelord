use crate::button::components::{Button, ButtonContent, ButtonVariant};
use crate::icon::components::{Icon, Language, Structure};
use crate::ui::component::Clickable;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;

use std::path::PathBuf;

/// File entry component for explorer tree.
#[derive(Component, Debug, Clone)]
pub struct FileEntry {
  pub path: PathBuf,
  pub parent: Option<PathBuf>,
  pub is_dir: bool,
  pub is_hidden: bool,
  pub depth: u32,
}

impl FileEntry {
  pub fn new(path: PathBuf, parent: Option<PathBuf>, depth: u32) -> Self {
    let is_dir = path.is_dir();
    let is_hidden = path
      .file_name()
      .map(|n| n.to_string_lossy().starts_with('.'))
      .unwrap_or(false);
    Self {
      path,
      parent,
      is_dir,
      is_hidden,
      depth,
    }
  }

  pub fn name(&self) -> String {
    self
      .path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default()
  }
}

/// Marker for expanded directories.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Expanded;

/// Marker for selected entry.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Selected;

/// Bundle for spawning file entry entities.
#[derive(Bundle)]
pub struct FileEntryBundle {
  pub entry: FileEntry,
  pub button: Button,
  pub clickable: Clickable,
}

impl FileEntryBundle {
  pub fn new(path: PathBuf, parent: Option<PathBuf>, depth: u32) -> Self {
    let is_dir = path.is_dir();
    let name = path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();
    let is_hidden = name.starts_with('.');

    let icon = if is_dir {
      Icon::Structure(Structure::FolderClose)
    } else {
      icon_for_file(&name)
    };

    // Leak the string to get a &'static str
    let label: &'static str = Box::leak(name.into_boxed_str());

    Self {
      entry: FileEntry {
        path,
        parent,
        is_dir,
        is_hidden,
        depth,
      },
      button: Button {
        content: ButtonContent::IconLabel(icon, label),
        variant: ButtonVariant::Ghost,
      },
      clickable: Clickable::default(),
    }
  }
}

/// Returns the appropriate icon for a file based on its name/extension.
fn icon_for_file(name: &str) -> Icon {
  let lower = name.to_lowercase();

  // Special filenames first
  if lower == "dockerfile"
    || lower.starts_with("dockerfile.")
    || lower.starts_with("docker-compose")
    || lower.starts_with("docker-stack")
  {
    return Icon::Language(Language::Docker);
  }
  if lower == "vite.config.ts"
    || lower == "vite.config.js"
    || lower == "vite.config.mts"
  {
    return Icon::Language(Language::Vite);
  }
  if lower == "vitest.config.ts"
    || lower == "vitest.config.js"
    || lower == "vitest.config.mts"
  {
    return Icon::Language(Language::Vitest);
  }
  if lower == ".gitignore" {
    return Icon::Language(Language::GitIgnore);
  }
  if lower == "license"
    || lower == "license.md"
    || lower == "license.txt"
    || lower == "license-mit"
    || lower == "license-apache"
  {
    return Icon::Language(Language::License);
  }
  if lower == "makefile" || lower == "gnumakefile" {
    return Icon::Language(Language::Makefile);
  }
  if lower == "favicon.ico" || lower == "favicon.png" || lower == "favicon.svg"
  {
    return Icon::Language(Language::Favicon);
  }
  if lower == ".env" || lower.starts_with(".env.") {
    return Icon::Language(Language::Env);
  }

  // Get extension
  let ext = lower.rsplit('.').next().unwrap_or("");

  match ext {
    "sh" | "bash" => Icon::Language(Language::Bash),
    "c" => Icon::Language(Language::C),
    "clj" | "cljs" | "cljc" | "edn" => Icon::Language(Language::Clojure),
    "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" => {
      Icon::Language(Language::Cplusplus)
    }
    "css" => Icon::Language(Language::Css),
    "csv" => Icon::Language(Language::Csv),
    "dart" => Icon::Language(Language::Dart),
    "ex" | "exs" | "heex" => Icon::Language(Language::Elixir),
    "erl" | "hrl" => Icon::Language(Language::Erlang),
    "ttf" | "otf" | "woff" | "woff2" | "eot" => Icon::Language(Language::Font),
    "gleam" => Icon::Language(Language::Gleam),
    "go" => Icon::Language(Language::Go),
    "html" | "htm" => Icon::Language(Language::Html),
    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tiff"
    | "tif" | "heic" | "heif" | "avif" => Icon::Language(Language::Image),
    "svg" => Icon::Language(Language::Svg),
    "js" | "mjs" | "cjs" => Icon::Language(Language::JavaScript),
    "json" | "jsonc" => Icon::Language(Language::Json),
    "kt" | "kts" => Icon::Language(Language::Kotlin),
    "lock" => Icon::Language(Language::Lock),
    "md" | "mdx" | "markdown" => Icon::Language(Language::Markdown),
    "ml" | "mli" => Icon::Language(Language::Ocaml),
    "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "wma" | "aiff"
    | "opus" => Icon::Language(Language::Music),
    "nim" | "nims" => Icon::Language(Language::Nim),
    "pdf" => Icon::Language(Language::Pdf),
    "py" | "pyw" | "pyi" => Icon::Language(Language::Python),
    "jsx" | "tsx" => Icon::Language(Language::React),
    "rb" | "erb" | "rake" => Icon::Language(Language::Ruby),
    "rs" => Icon::Language(Language::Rust),
    "sass" | "scss" => Icon::Language(Language::Sass),
    "sqlite" | "sqlite3" | "db" => Icon::Language(Language::Sqlite),
    "sql" => Icon::Language(Language::Database),
    "svelte" => Icon::Language(Language::Svelte),
    "toml" => Icon::Language(Language::Toml),
    "ts" | "mts" | "cts" => Icon::Language(Language::TypeScript),
    "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" | "m4v" => {
      Icon::Language(Language::Video)
    }
    "vue" => Icon::Language(Language::Vue),
    "wasm" => Icon::Language(Language::Wasm),
    "wat" => Icon::Language(Language::Wat),
    "xls" | "xlsx" | "xlsm" | "xlsb" => Icon::Language(Language::Excel),
    "yaml" | "yml" => Icon::Language(Language::Yaml),
    "zig" => Icon::Language(Language::Zig),
    "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => {
      Icon::Language(Language::Zip)
    }
    _ => Icon::Structure(Structure::File),
  }
}
