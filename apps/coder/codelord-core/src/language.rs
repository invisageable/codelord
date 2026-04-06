//! Language types for syntax highlighting and symbol extraction.

/// Represents the language type for a source file.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
  /// The `bash` language type.
  Bash,
  /// The `c` language type.
  C,
  /// The `css` language type.
  Css,
  /// The `conf` language type (.conf files).
  Conf,
  /// The `csv` language type.
  Csv,
  /// The `diff` language type (git diffs, patches).
  Diff,
  /// The `elixir` language type.
  Elixir,
  /// The `env` language type (.env files).
  Env,
  /// The `gleam` language type.
  Gleam,
  /// The `go` language type.
  Go,
  /// The `html` language type.
  Html,
  /// The `image` language type (PNG, JPEG, GIF, WebP, etc.).
  Image,
  /// The `javascript` language type.
  JavaScript,
  /// The `json` language type.
  Json,
  /// The `markdown` language type.
  Markdown,
  /// The `ocaml` language type.
  Ocaml,
  /// The `plain text` language type.
  #[default]
  PlainText,
  /// The `python` language type.
  Python,
  /// The `rust` language type.
  Rust,
  /// The `toml` language type.
  Toml,
  /// The `typescript` language type.
  TypeScript,
  /// The `yaml` language type.
  Yaml,
  /// The `zig` language type.
  Zig,
  /// The `zo` language type.
  Zo,
}

impl Language {
  /// Get the line comment prefix for this language.
  pub fn line_comment_prefix(&self) -> &'static str {
    match self {
      Self::Bash => "# ",
      Self::C => "// ",
      Self::Conf => "# ",
      Self::Css => "/* ",
      Self::Csv => "",
      Self::Diff => "",
      Self::Elixir => "# ",
      Self::Env => "# ",
      Self::Gleam => "// ",
      Self::Image => "",
      Self::Go => "// ",
      Self::Html => "",
      Self::JavaScript => "// ",
      Self::Json => "",
      Self::Markdown => "",
      Self::Ocaml => "(* ",
      Self::PlainText => "// ",
      Self::Python => "# ",
      Self::Rust => "// ",
      Self::Toml => "# ",
      Self::TypeScript => "// ",
      Self::Yaml => "# ",
      Self::Zig => "// ",
      Self::Zo => "-- ",
    }
  }

  /// Check if this language supports line comments.
  pub fn supports_line_comments(&self) -> bool {
    !matches!(
      self,
      Self::Csv
        | Self::Diff
        | Self::Image
        | Self::Json
        | Self::Markdown
        | Self::PlainText
        | Self::Html
    )
  }
}

impl From<&str> for Language {
  fn from(ext: &str) -> Self {
    match ext {
      "sh" | "bash" => Self::Bash,
      "c" | "h" => Self::C,
      "conf" => Self::Conf,
      "css" => Self::Css,
      "csv" => Self::Csv,
      "diff" | "patch" => Self::Diff,
      "env" => Self::Env,
      "ex" | "exs" => Self::Elixir,
      "gleam" => Self::Gleam,
      "go" => Self::Go,
      "html" | "htm" => Self::Html,
      "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => Self::Image,
      "js" | "mjs" | "cjs" => Self::JavaScript,
      "json" => Self::Json,
      "md" | "markdown" => Self::Markdown,
      "ml" | "mli" => Self::Ocaml,
      "py" => Self::Python,
      "rs" => Self::Rust,
      "toml" => Self::Toml,
      "ts" | "mts" | "cts" => Self::TypeScript,
      "yaml" | "yml" => Self::Yaml,
      "zig" => Self::Zig,
      "zo" => Self::Zo,
      _ => Self::PlainText,
    }
  }
}
