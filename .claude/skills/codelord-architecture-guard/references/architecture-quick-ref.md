# Codelord Architecture Quick Reference

## Core Philosophy

1. **Voice-first**: Voice control is the primary power interface. Push-to-talk. Whisper local. Regex offline fallback. Every new feature must be voice-controllable.
2. **Held-key modality**: Hold Space = command mode. Release = typing. No toggle modes. Physical feedback of mode, like game controllers.
3. **Zero-config**: Works out of the box. Visual settings UI for 80%. Voice config for 15%. File config for 5%. No config-file-only options.
4. **Copilord**: Local AI assistant. Privacy-first. No subscription. Panel slides open contextually.

## Crate Dependency Rules

```
codelord (binary)
  -> codelord-core (ECS state + logic)
  -> codelord-components (egui UI) -> depends on codelord-core
  -> codelord-runtime

codelord-core
  -> codelord-language (tree-sitter)
  -> codelord-audio (rodio/cpal)
  -> codelord-voice (whisper-rs)
  -> codelord-git (git ops)
  -> codelord-i18n (localization)
  -> codelord-protocol (shared DTOs)
  -> codelord-sdk (PDF/SQLite/voice bindings)
  -> zo-* (compiler crates)

codelord-server (independent, no UI deps)
  -> codelord-protocol
  -> async-openai, axum
```

NEVER: core -> components, server -> components, circular deps.

## ECS Feature Module Pattern

```
feature/
  mod.rs            # re-exports
  components.rs     # plain data (#[derive(Component)]), NO logic
  resources.rs      # shared state (#[derive(Resource)])
  systems.rs        # ALL logic here
  events.rs         # optional (#[derive(Event)])
  bundles.rs        # optional (#[derive(Bundle)])
```

## UI Component Hierarchy

```
atoms/        # buttons, icons, badges
molecules/    # composed atoms
organisms/    # titlebar, statusbar
pages/        # text editor, terminal, settings, welcome
panels/       # explorer, search, music player
views/        # copilord chat, editor content
overlays/     # file picker, popups, toasts
```

## Data Flow (One-Way, Never Reverse)

```
User Input -> Events -> Systems -> Resources -> UI Components -> egui
                                      |
                                      v
                              Server (async, via channels)
```

## Performance Rules

1. No allocations in `fn update()` or `fn show()` hot paths
2. Cache computed values in Resources, invalidate on change
3. `SmallVec<[T; N]>` for small bounded collections
4. `&str` over `String` in UI rendering
5. Batch tree-sitter queries, don't re-parse unchanged buffers
6. Enums for closed sets, no `Box<dyn Trait>` in hot paths
7. Animation uses fixed-timestep spring physics

## Voice Command Architecture

```
Audio Input -> Whisper (local) -> Text -> OpenAI/Regex -> VoiceCommand -> ECS System
```

- 32+ commands implemented (tabs, panels, navigation, compiler stages)
- Regex fallback for offline use
- New features MUST add voice commands
