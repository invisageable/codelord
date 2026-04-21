# Codelord

A voice-first, held-key modal code editor. Rust, egui + wgpu, bevy_ecs.

## Philosophy

- **Voice-first**: Voice control is the primary power interface, not an accessory. Push-to-talk via key. Whisper runs locally. Regex fallback for offline.
- **Held-key modality**: No toggle modes (no vim-style i/Esc). Hold Space = command mode. Release = back to typing. Physical feedback of mode. Like game controllers.
- **Zero-config**: Works out of the box. Visual settings UI. Voice-configurable. Config files only for the 5% who want them.
- **Copilord**: Local AI assistant. Privacy-first. No subscription. Panel slides open with suggestions.
- **zo integration**: Native zo compiler support. Live SIR view. Stage scrubber. Voice-driven compilation.

## Architecture

### Two-Layer ECS

**`codelord-core`** (state + logic) and **`codelord-components`** (UI rendering). One-way dependency: components depends on core, never reverse.

### ECS Feature Module Pattern

Every feature in `codelord-core/src/` follows:

```
feature/
  mod.rs            # re-exports
  components.rs     # plain data structs (#[derive(Component)]), NO logic
  resources.rs      # shared mutable state (#[derive(Resource)])
  systems.rs        # ALL logic lives here, operates on components/resources
  events.rs         # optional: custom events (#[derive(Event)])
  bundles.rs        # optional: convenience spawners (#[derive(Bundle)])
```

Rules:
- Components are pure data. No methods with side effects.
- Resources hold shared state. Must be `Send + Sync` if cross-thread.
- Systems contain all logic. Read components/resources, produce effects.
- UI components receive data, render, return responses. No state ownership.

### UI Component Hierarchy (Atomic Design)

```
atoms/       # buttons, icons, badges
molecules/   # composed atoms
organisms/   # titlebar, statusbar
pages/       # text editor, terminal, settings, welcome
panels/      # explorer, search, music player
views/       # copilord chat, editor content
overlays/    # filescope, popups, toasts
```

### Data Flow

```
User Input -> Events -> Systems -> Resources -> UI Components -> egui
                                      |
                                      v
                              Server (async, via channels)
```

One-way: Systems mutate Resources. UI reads Resources. Never reverse.

### Crate Map

| Crate | Role |
|-------|------|
| `codelord` | Binary entry point, app loop |
| `codelord-core` | ECS state, logic, all feature modules |
| `codelord-components` | egui UI rendering layer |
| `codelord-language` | Tree-sitter syntax highlighting, per-language configs |
| `codelord-audio` | Music player, SFX (rodio/cpal) |
| `codelord-voice` | Voice control (whisper-rs transcription, command parsing) |
| `codelord-git` | Git blame, branch ops |
| `codelord-protocol` | Serializable DTOs shared between client and server |
| `codelord-sdk` | PDF, SQLite, voice bindings |
| `codelord-server` | Axum backend: OpenAI/Copilord, voice, playground routes |
| `codelord-i18n` | Localization (en, fr, ja, zh-CN) |
| `codelord-runtime` | Runtime abstraction |

### Crate Boundary Rules

- `codelord-core` does NOT depend on `codelord-components`
- `codelord-protocol` contains ONLY serializable DTOs
- `codelord-server` does NOT depend on UI crates
- No circular dependencies

## Code Conventions

- 2-space indentation, 80-char line width
- No trait objects in hot paths. Enums for closed sets.
- No allocations in render loops or per-frame paths
- `SmallVec`/`ThinVec` for small collections, arenas for tree structures
- Cache computed values in Resources, invalidate on change
- Prefer `&str` over `String` in UI rendering

## Build Commands

```sh
just build          # Build all targets
just test           # Run all tests
just clippy         # Clippy with -D warnings
just fmt            # Format all code
just fmt_check      # Check formatting
just pre-commit     # fmt_check + clippy + test
just ci             # Full CI simulation (includes Linux Docker)
```

## Key Systems

- **Animation**: Spring physics, tweening, glow, shimmer (`codelord-core/src/animation/`)
- **Voice**: 32+ commands, Whisper local, regex offline fallback (`codelord-voice/`, `codelord-core/src/voice/`)
- **Theme**: Hot-reload, multiple themes (`codelord-core/src/theme/`)
- **Filescope**: Fuzzy search via `nucleo` (`codelord-core/src/filescope/`)
- **XMB navigation**: PlayStation-style menu system (`codelord-core/src/xmb/`)
- **Drag-and-drop**: With animation (`codelord-core/src/drag_and_drop/`)
- **Terminal**: Alacritty-based with custom cursor animation (`codelord-components/src/components/pages/terminal/`)
- **Renderers**: CSV, PDF, Markdown, SQLite, SVG, XLS, font preview, webview

## Architecture Notes

See `apps/coder/codelord-notes/personal/architecture/` for detailed design docs:
- `IDE_STRUCTURE.md` - Canonical ECS module pattern
- `IDE_FEATURE_STRUCTURE.md` - Feature-based directory layout
- `CODELORD_UX_VISION.md` - Voice-first philosophy, held-key system, personas
