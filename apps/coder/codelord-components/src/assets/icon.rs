use crate::assets::image::image_from_source;

use codelord_core::icon::components::{
  Arrow, Byakugan, Dot, Download, Feedback, Folder, Icon, Key, Language,
  Layout, Player, Preview, Structure, Zoom,
};

use eframe::egui;

/// Converts Icon component to egui::Image
pub fn icon_to_image(icon: &Icon) -> egui::Image<'static> {
  image_from_source(icon_to_source(icon))
}

/// Converts Icon component to egui::ImageSource
fn icon_to_source(icon: &Icon) -> egui::ImageSource<'static> {
  match icon {
    Icon::Add => {
      egui::include_image!("../../../codelord-assets/icon/icon-add.svg")
    }
    Icon::Alien => {
      egui::include_image!(
        "../../../codelord-assets/icon/alien/icon-alien-hand.svg"
      )
    }
    Icon::Arrow(Arrow::DoubleRight) => {
      egui::include_image!(
        "../../../codelord-assets/icon/arrow/icon-arrow-double-right.svg"
      )
    }
    Icon::Arrow(Arrow::Left) => {
      egui::include_image!(
        "../../../codelord-assets/icon/arrow/icon-arrow-left.svg"
      )
    }
    Icon::Arrow(Arrow::Right) => {
      egui::include_image!(
        "../../../codelord-assets/icon/arrow/icon-arrow-right.svg"
      )
    }
    Icon::Arrow(Arrow::AngleLeftLine) => {
      egui::include_image!(
        "../../../codelord-assets/icon/arrow/icon-arrow-angle-left-line.svg"
      )
    }
    Icon::Arrow(Arrow::AngleRightLine) => {
      egui::include_image!(
        "../../../codelord-assets/icon/arrow/icon-arrow-angle-right-line.svg"
      )
    }
    Icon::Binary => {
      egui::include_image!("../../../codelord-assets/icon/icon-binary.svg")
    }
    Icon::Browser => {
      egui::include_image!("../../../codelord-assets/icon/icon-browser.svg")
    }
    Icon::Byakugan(Byakugan::On) => {
      egui::include_image!(
        "../../../codelord-assets/icon/byakugan/icon-byakugan-on.svg"
      )
    }
    Icon::Byakugan(Byakugan::Off) => {
      egui::include_image!(
        "../../../codelord-assets/icon/byakugan/icon-byakugan-off.svg"
      )
    }
    Icon::Close => {
      egui::include_image!("../../../codelord-assets/icon/icon-close.svg")
    }
    Icon::Code => {
      egui::include_image!("../../../codelord-assets/icon/icon-code.svg")
    }
    Icon::Collapse => {
      egui::include_image!("../../../codelord-assets/icon/icon-collapse.svg")
    }
    Icon::Copilord => {
      egui::include_image!("../../../codelord-assets/icon/icon-copilord.svg")
    }
    Icon::Dot(Dot::Horizontal) => {
      egui::include_image!(
        "../../../codelord-assets/icon/dot/icon-dot-horizontal.svg"
      )
    }
    Icon::Download(Download::Cloud) => {
      egui::include_image!(
        "../../../codelord-assets/icon/download/icon-download-cloud.svg"
      )
    }
    Icon::Download(Download::Folder) => {
      egui::include_image!(
        "../../../codelord-assets/icon/download/icon-download-folder.svg"
      )
    }
    Icon::Explorer => {
      egui::include_image!("../../../codelord-assets/icon/icon-files.svg")
    }
    Icon::Feedback(Feedback::Alert) => {
      egui::include_image!(
        "../../../codelord-assets/icon/feedback/icon-feedback-alert.svg"
      )
    }
    Icon::Feedback(Feedback::Info) => {
      egui::include_image!(
        "../../../codelord-assets/icon/feedback/icon-feedback-info.svg"
      )
    }
    Icon::Feedback(Feedback::Success) => {
      egui::include_image!(
        "../../../codelord-assets/icon/feedback/icon-feedback-success.svg"
      )
    }
    Icon::Feedback(Feedback::SuccessRounded) => {
      egui::include_image!(
        "../../../codelord-assets/icon/feedback/icon-feedback-success-rounded.svg"
      )
    }
    Icon::Feedback(Feedback::Warning) => {
      egui::include_image!(
        "../../../codelord-assets/icon/feedback/icon-feedback-warning.svg"
      )
    }
    Icon::Folder(Folder::Open) => {
      egui::include_image!(
        "../../../codelord-assets/icon/folder/icon-folder-open.svg"
      )
    }
    Icon::Folder(Folder::Close) => {
      egui::include_image!(
        "../../../codelord-assets/icon/folder/icon-folder-close.svg"
      )
    }
    Icon::Hacker => {
      egui::include_image!("../../../codelord-assets/icon/icon-hacker.svg")
    }
    Icon::Home => {
      egui::include_image!("../../../codelord-assets/icon/icon-home.svg")
    }
    Icon::Key(Key::Alt) => {
      egui::include_image!("../../../codelord-assets/icon/key/icon-key-alt.svg")
    }
    Icon::Key(Key::Backspace) => {
      egui::include_image!(
        "../../../codelord-assets/icon/key/icon-key-backspace.svg"
      )
    }
    Icon::Key(Key::Cmd) => {
      egui::include_image!("../../../codelord-assets/icon/key/icon-key-cmd.svg")
    }
    Icon::Key(Key::Enter) => {
      egui::include_image!(
        "../../../codelord-assets/icon/key/icon-key-enter.svg"
      )
    }
    Icon::Key(Key::Shift) => {
      egui::include_image!(
        "../../../codelord-assets/icon/key/icon-key-shift.svg"
      )
    }
    Icon::Key(Key::Space) => {
      egui::include_image!(
        "../../../codelord-assets/icon/key/icon-key-space-bar.svg"
      )
    }
    Icon::Key(Key::Tab) => {
      egui::include_image!("../../../codelord-assets/icon/key/icon-key-tab.svg")
    }
    Icon::Keyboard => {
      egui::include_image!("../../../codelord-assets/icon/icon-keyboard.svg")
    }
    Icon::Language(Language::Bash) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-bash.svg"
      )
    }
    Icon::Language(Language::C) => {
      egui::include_image!("../../../codelord-assets/icon/language/icon-c.svg")
    }
    Icon::Language(Language::Clojure) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-clojure.svg"
      )
    }
    Icon::Language(Language::Cplusplus) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-cplusplus.svg"
      )
    }
    Icon::Language(Language::Css) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-css3.svg"
      )
    }
    Icon::Language(Language::Csv) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-csv.svg"
      )
    }
    Icon::Language(Language::Dart) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-dart.svg"
      )
    }
    Icon::Language(Language::Database) => {
      egui::include_image!("../../../codelord-assets/icon/language/icon-db.svg")
    }
    Icon::Language(Language::Docker) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-docker.svg"
      )
    }
    Icon::Language(Language::Elixir) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-elixir.svg"
      )
    }
    Icon::Language(Language::Env) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-env.svg"
      )
    }
    Icon::Language(Language::Erlang) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-erlang.svg"
      )
    }
    Icon::Language(Language::Excel) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-xls.svg"
      )
    }
    Icon::Language(Language::Favicon) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-favicon.svg"
      )
    }
    Icon::Language(Language::Font) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-font.svg"
      )
    }
    Icon::Language(Language::GitIgnore) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-git-ignore.svg"
      )
    }
    Icon::Language(Language::Github) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-github.svg"
      )
    }
    Icon::Language(Language::Gleam) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-gleam.svg"
      )
    }
    Icon::Language(Language::Go) => {
      egui::include_image!("../../../codelord-assets/icon/language/icon-go.svg")
    }
    Icon::Language(Language::Html) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-html5.svg"
      )
    }
    Icon::Language(Language::Image) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-image.svg"
      )
    }
    Icon::Language(Language::JavaScript) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-javascript.svg"
      )
    }
    Icon::Language(Language::Json) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-json.svg"
      )
    }
    Icon::Language(Language::Kotlin) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-kotlin.svg"
      )
    }
    Icon::Language(Language::License) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-license.svg"
      )
    }
    Icon::Language(Language::Lock) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-lock.svg"
      )
    }
    Icon::Language(Language::Makefile) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-makefile.svg"
      )
    }
    Icon::Language(Language::Markdown) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-markdown.svg"
      )
    }
    Icon::Language(Language::Music) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-music.svg"
      )
    }
    Icon::Language(Language::Nim) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-nim.svg"
      )
    }
    Icon::Language(Language::Ocaml) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-ocaml.svg"
      )
    }
    Icon::Language(Language::Pdf) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-pdf.svg"
      )
    }
    Icon::Language(Language::Python) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-python.svg"
      )
    }
    Icon::Language(Language::React) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-react.svg"
      )
    }
    Icon::Language(Language::Ruby) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-ruby.svg"
      )
    }
    Icon::Language(Language::Rust) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-rust.svg"
      )
    }
    Icon::Language(Language::Sass) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-sass.svg"
      )
    }
    Icon::Language(Language::Sqlite) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-sqlite.svg"
      )
    }
    Icon::Language(Language::Svg) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-svg.svg"
      )
    }
    Icon::Language(Language::Svelte) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-svelte.svg"
      )
    }
    Icon::Language(Language::Toml) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-toml.svg"
      )
    }
    Icon::Language(Language::TypeScript) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-typescript.svg"
      )
    }
    Icon::Language(Language::Video) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-video.svg"
      )
    }
    Icon::Language(Language::Vite) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-vitejs.svg"
      )
    }
    Icon::Language(Language::Vitest) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-vitest.svg"
      )
    }
    Icon::Language(Language::Vue) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-vuejs.svg"
      )
    }
    Icon::Language(Language::Wasm) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-wasm.svg"
      )
    }
    Icon::Language(Language::Wat) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-wat.svg"
      )
    }
    Icon::Language(Language::Yaml) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-yaml.svg"
      )
    }
    Icon::Language(Language::Zig) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-zig.svg"
      )
    }
    Icon::Language(Language::Zip) => {
      egui::include_image!(
        "../../../codelord-assets/icon/language/icon-zip.svg"
      )
    }
    Icon::Layout(Layout::Custom) => {
      egui::include_image!(
        "../../../codelord-assets/icon/layout/icon-layout-custom.svg"
      )
    }
    Icon::Notes => {
      egui::include_image!("../../../codelord-assets/icon/icon-notes.svg")
    }
    Icon::Player(Player::MusicNote) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-music-note.svg"
      )
    }
    Icon::Player(Player::Muted) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-muted.svg"
      )
    }
    Icon::Player(Player::Next) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-next.svg"
      )
    }
    Icon::Player(Player::Pause) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-pause.svg"
      )
    }
    Icon::Player(Player::Play) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-play.svg"
      )
    }
    Icon::Player(Player::Playlist) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-playlist.svg"
      )
    }
    Icon::Player(Player::Replay) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-replay.svg"
      )
    }
    Icon::Player(Player::Stop) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-stop.svg"
      )
    }
    Icon::Player(Player::Volume) => {
      egui::include_image!(
        "../../../codelord-assets/icon/player/icon-player-volume.svg"
      )
    }
    Icon::Preview(Preview::Markdown) => {
      egui::include_image!(
        "../../../codelord-assets/icon/preview/icon-preview-markdown.svg"
      )
    }
    Icon::Quote => {
      egui::include_image!("../../../codelord-assets/icon/icon-quote.svg")
    }
    Icon::Refresh => {
      egui::include_image!("../../../codelord-assets/icon/icon-refresh.svg")
    }
    Icon::Schema => {
      egui::include_image!("../../../codelord-assets/icon/icon-schema.svg")
    }
    Icon::Search => {
      egui::include_image!("../../../codelord-assets/icon/icon-search.svg")
    }
    Icon::Server => {
      egui::include_image!("../../../codelord-assets/icon/icon-server.svg")
    }
    Icon::Sound => {
      egui::include_image!("../../../codelord-assets/icon/icon-sound.svg")
    }
    Icon::Structure(Structure::File) => {
      egui::include_image!("../../../codelord-assets/icon/icon-file.svg")
    }
    Icon::Structure(Structure::FolderOpen) => {
      egui::include_image!(
        "../../../codelord-assets/icon/folder/icon-folder-open.svg"
      )
    }
    Icon::Structure(Structure::FolderClose) => {
      egui::include_image!(
        "../../../codelord-assets/icon/folder/icon-folder-close.svg"
      )
    }
    Icon::Table => {
      egui::include_image!("../../../codelord-assets/icon/icon-table.svg")
    }
    Icon::Terminal => {
      egui::include_image!("../../../codelord-assets/icon/icon-terminal.svg")
    }
    Icon::Theme => {
      egui::include_image!("../../../codelord-assets/icon/icon-theme.svg")
    }
    Icon::Ufo => {
      egui::include_image!("../../../codelord-assets/icon/icon-ufo.svg")
    }
    Icon::Voice => {
      egui::include_image!("../../../codelord-assets/icon/icon-voice.svg")
    }
    Icon::Zoom(Zoom::InArrow) => {
      egui::include_image!(
        "../../../codelord-assets/icon/zoom/icon-zoom-in-arrow.svg"
      )
    }
    Icon::Zoom(Zoom::OutArrow) => {
      egui::include_image!(
        "../../../codelord-assets/icon/zoom/icon-zoom-out-arrow.svg"
      )
    }
  }
}
