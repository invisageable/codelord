//! Terminal bridge - Alacritty integration for ECS.
//!
//! This bridge wraps alacritty_terminal and syncs state to ECS components.

use codelord_core::terminal::{
  CELL_FLAG_BOLD, CELL_FLAG_DIM, CELL_FLAG_ITALIC, CELL_FLAG_REVERSE,
  CELL_FLAG_STRIKETHROUGH, CELL_FLAG_UNDERLINE, TerminalCell, TerminalGrid,
  pack_color,
};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::tty::{self, Options, Shell};
use alacritty_terminal::vte::ansi::{ClearMode, Color, Handler, NamedColor};

use flume::{Receiver, Sender};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Allowed shells - must be in this whitelist and exist on disk.
const ALLOWED_SHELLS: &[&str] = &[
  "/bin/sh",
  "/bin/bash",
  "/bin/zsh",
  "/usr/bin/bash",
  "/usr/bin/zsh",
  "/usr/local/bin/bash",
  "/usr/local/bin/zsh",
  "/opt/homebrew/bin/bash",
  "/opt/homebrew/bin/zsh",
];

/// Environment variables allowed to pass to spawned shell.
/// Excludes credentials like AWS_SECRET_KEY, SSH_AUTH_SOCK, etc.
const ALLOWED_ENV_VARS: &[&str] = &[
  "PATH",
  "HOME",
  "USER",
  "SHELL",
  "EDITOR",
  "VISUAL",
  "XDG_CONFIG_HOME",
  "XDG_DATA_HOME",
  "XDG_CACHE_HOME",
];

/// Returns a validated shell path.
///
/// Checks the SHELL environment variable against a whitelist
/// and verifies the shell exists on disk.
fn get_validated_shell() -> String {
  if let Ok(shell) = std::env::var("SHELL")
    && ALLOWED_SHELLS.contains(&shell.as_str())
    && Path::new(&shell).exists()
  {
    return shell;
  }

  // Fallback to first available shell
  for shell in ALLOWED_SHELLS {
    if Path::new(shell).exists() {
      return shell.to_string();
    }
  }

  "/bin/sh".to_string()
}

/// Snapshot of terminal state for rendering.
pub struct RenderableContent {
  pub grid: TerminalGrid,
  pub cursor_row: u16,
  pub cursor_col: u16,
  pub cursor_visible: bool,
  pub total_lines: usize,
  pub display_offset: usize,
}

impl Default for RenderableContent {
  fn default() -> Self {
    Self {
      grid: TerminalGrid::default(),
      cursor_row: 0,
      cursor_col: 0,
      cursor_visible: true,
      total_lines: 24,
      display_offset: 0,
    }
  }
}

/// Event proxy for Alacritty events.
#[derive(Clone)]
struct EventProxy(Sender<Event>);

impl EventListener for EventProxy {
  fn send_event(&self, event: Event) {
    let _ = self.0.send(event);
  }
}

/// Alacritty terminal bridge.
pub struct AlacrittyBridge {
  term: Arc<FairMutex<Term<EventProxy>>>,
  event_rx: Receiver<Event>,
  event_loop_sender: Option<EventLoopSender>,
}

impl AlacrittyBridge {
  /// Create a new terminal bridge.
  pub fn new(
    rows: u16,
    cols: u16,
    working_directory: Option<PathBuf>,
  ) -> Result<Self, Box<dyn std::error::Error>> {
    let (event_tx, event_rx) = flume::unbounded();
    let config = Config::default();
    let event_proxy = EventProxy(event_tx);

    struct TermDimensions {
      columns: usize,
      lines: usize,
    }

    impl Dimensions for TermDimensions {
      fn columns(&self) -> usize {
        self.columns
      }
      fn screen_lines(&self) -> usize {
        self.lines
      }
      fn total_lines(&self) -> usize {
        self.lines
      }
    }

    let dimensions = TermDimensions {
      columns: cols as usize,
      lines: rows as usize,
    };

    let term = Term::new(config, &dimensions, event_proxy.clone());

    let shell = get_validated_shell();

    // Explicit environment - only pass safe variables to prevent credential
    // leakage
    let mut env = HashMap::new();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("PROMPT_EOL_MARK".to_string(), "".to_string());
    env.insert("LANG".to_string(), "en_US.UTF-8".to_string());
    env.insert("LC_ALL".to_string(), "en_US.UTF-8".to_string());

    // Pass through only safe environment variables
    for var in ALLOWED_ENV_VARS {
      if let Ok(value) = std::env::var(var) {
        env.insert(var.to_string(), value);
      }
    }

    let options = Options {
      shell: Some(Shell::new(shell, vec![])),
      working_directory: Some(
        working_directory.unwrap_or_else(|| PathBuf::from("/")),
      ),
      drain_on_exit: true,
      #[cfg(windows)]
      escape_args: false,
      env,
    };

    let window_size = WindowSize {
      num_lines: rows,
      num_cols: cols,
      cell_width: 8,
      cell_height: 16,
    };

    let pty = tty::new(&options, window_size, 0)?;
    let term = Arc::new(FairMutex::new(term));

    let event_loop =
      EventLoop::new(Arc::clone(&term), event_proxy, pty, false, false)?;

    let event_loop_sender = event_loop.channel();

    event_loop.spawn();

    let bridge = Self {
      term,
      event_rx,
      event_loop_sender: Some(event_loop_sender),
    };

    // Wait for shell initialization
    std::thread::sleep(std::time::Duration::from_millis(100));

    bridge.clear_screen();

    Ok(bridge)
  }

  /// Send input to terminal (keystrokes, paste).
  pub fn send_input(&self, input: &str) {
    if let Some(sender) = self.event_loop_sender.as_ref() {
      let bytes = input.as_bytes().to_vec();
      let _ = sender.send(Msg::Input(bytes.into()));
    }
  }

  /// Resize terminal.
  pub fn resize(&self, rows: u16, cols: u16) {
    struct TermSize {
      columns: usize,
      lines: usize,
    }

    impl Dimensions for TermSize {
      fn columns(&self) -> usize {
        self.columns
      }
      fn screen_lines(&self) -> usize {
        self.lines
      }
      fn total_lines(&self) -> usize {
        self.lines
      }
    }

    // Resize internal terminal grid
    let mut term = self.term.lock();

    term.resize(TermSize {
      columns: cols as usize,
      lines: rows as usize,
    });

    drop(term);

    // Notify PTY of size change (sends SIGWINCH to shell)
    if let Some(sender) = self.event_loop_sender.as_ref() {
      let window_size = WindowSize {
        num_lines: rows,
        num_cols: cols,
        cell_width: 8,
        cell_height: 16,
      };

      let _ = sender.send(Msg::Resize(window_size));
    }
  }

  /// Scroll display.
  pub fn scroll(&self, lines: i32) {
    let mut term = self.term.lock();

    term.scroll_display(Scroll::Delta(lines));
  }

  /// Clear screen.
  pub fn clear_screen(&self) {
    let mut term = self.term.lock();
    term.clear_screen(ClearMode::Saved);

    let cursor = term.grid().cursor.point;
    term.grid_mut().reset_region(..cursor.line);
  }

  /// Process pending events from PTY.
  pub fn process_events(&self) {
    while let Ok(event) = self.event_rx.try_recv() {
      match event {
        Event::Wakeup => {}
        Event::PtyWrite(text) => {
          self
            .event_loop_sender
            .as_ref()
            .map(|sender| sender.send(Msg::Input(text.into_bytes().into())));
        }
        _ => {}
      }
    }
  }

  /// Sync terminal state and return content for ECS update.
  ///
  /// Returns the current terminal state directly - no Arc<Mutex> caching.
  /// The caller should update ECS components with the returned content.
  pub fn sync(&self) -> RenderableContent {
    let term = self.term.lock();
    let grid = term.grid();

    let total_lines = grid.total_lines();
    let display_offset = grid.display_offset();
    let cursor_row = grid.cursor.point.line.0 as u16;
    let cursor_col = grid.cursor.point.column.0 as u16;

    drop(term);

    RenderableContent {
      grid: self.grid_snapshot_internal(),
      total_lines,
      display_offset,
      cursor_row,
      cursor_col,
      cursor_visible: true,
    }
  }

  /// Create grid snapshot.
  fn grid_snapshot_internal(&self) -> TerminalGrid {
    let term = self.term.lock();
    let grid = term.grid();

    let width = grid.columns() as u16;
    let screen_lines = grid.screen_lines();
    let display_offset = grid.display_offset();

    let mut terminal_grid = TerminalGrid::new(width, screen_lines as u16);

    for indexed in grid.display_iter() {
      let flags = indexed.cell.flags;

      if flags.contains(Flags::WIDE_CHAR_SPACER) {
        continue;
      }

      let line_num = indexed.point.line.0 + display_offset as i32;
      let col = indexed.point.column.0 as u16;

      if line_num >= 0 && line_num < screen_lines as i32 && col < width {
        let row = line_num as u16;
        terminal_grid.set_cell(row, col, convert_cell(indexed.cell));
      }
    }

    terminal_grid
  }
}

/// Convert Alacritty cell to our TerminalCell.
fn convert_cell(cell: &Cell) -> TerminalCell {
  let fg = color_to_rgba(&cell.fg);
  let bg = color_to_rgba(&cell.bg);

  let mut flags = 0u8;
  if cell.flags.contains(Flags::BOLD) {
    flags |= CELL_FLAG_BOLD;
  }
  if cell.flags.contains(Flags::ITALIC) {
    flags |= CELL_FLAG_ITALIC;
  }
  if cell.flags.contains(Flags::UNDERLINE)
    || cell.flags.contains(Flags::DOUBLE_UNDERLINE)
  {
    flags |= CELL_FLAG_UNDERLINE;
  }
  if cell.flags.contains(Flags::STRIKEOUT) {
    flags |= CELL_FLAG_STRIKETHROUGH;
  }
  if cell.flags.contains(Flags::DIM) {
    flags |= CELL_FLAG_DIM;
  }
  if cell.flags.contains(Flags::INVERSE) {
    flags |= CELL_FLAG_REVERSE;
  }

  TerminalCell {
    character: cell.c,
    fg_color: fg,
    bg_color: bg,
    flags,
  }
}

/// Convert Alacritty color to RGBA.
fn color_to_rgba(color: &Color) -> u32 {
  match color {
    Color::Named(named) => named_color_to_rgba(*named),
    Color::Spec(rgb) => pack_color(rgb.r, rgb.g, rgb.b, 255),
    Color::Indexed(idx) => indexed_color_to_rgba(*idx),
  }
}

/// Named ANSI colors to RGBA.
fn named_color_to_rgba(color: NamedColor) -> u32 {
  match color {
    NamedColor::Black => pack_color(0, 0, 0, 255),
    NamedColor::Red => pack_color(205, 49, 49, 255),
    NamedColor::Green => pack_color(13, 188, 121, 255),
    NamedColor::Yellow => pack_color(229, 229, 16, 255),
    NamedColor::Blue => pack_color(36, 114, 200, 255),
    NamedColor::Magenta => pack_color(188, 63, 188, 255),
    NamedColor::Cyan => pack_color(17, 168, 205, 255),
    NamedColor::White => pack_color(229, 229, 229, 255),
    NamedColor::BrightBlack => pack_color(102, 102, 102, 255),
    NamedColor::BrightRed => pack_color(241, 76, 76, 255),
    NamedColor::BrightGreen => pack_color(35, 209, 139, 255),
    NamedColor::BrightYellow => pack_color(245, 245, 67, 255),
    NamedColor::BrightBlue => pack_color(59, 142, 234, 255),
    NamedColor::BrightMagenta => pack_color(214, 112, 214, 255),
    NamedColor::BrightCyan => pack_color(41, 184, 219, 255),
    NamedColor::BrightWhite => pack_color(255, 255, 255, 255),
    NamedColor::Foreground => pack_color(229, 229, 229, 255),
    NamedColor::Background => pack_color(0, 0, 0, 0),
    NamedColor::Cursor => pack_color(255, 255, 255, 255),
    _ => pack_color(229, 229, 229, 255),
  }
}

/// Indexed colors (0-255) to RGBA.
fn indexed_color_to_rgba(idx: u8) -> u32 {
  match idx {
    0..=15 => {
      // Standard ANSI colors
      let colors: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
      ];
      let (r, g, b) = colors[idx as usize];
      pack_color(r, g, b, 255)
    }
    16..=231 => {
      // 6x6x6 color cube
      let idx = idx - 16;
      let r = (idx / 36) % 6;
      let g = (idx / 6) % 6;
      let b = idx % 6;
      let to_255 = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
      pack_color(to_255(r), to_255(g), to_255(b), 255)
    }
    232..=255 => {
      // Grayscale
      let gray = 8 + (idx - 232) * 10;
      pack_color(gray, gray, gray, 255)
    }
  }
}
