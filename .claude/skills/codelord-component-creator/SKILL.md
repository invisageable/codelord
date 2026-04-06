---
name: codelord-component-creator
description: >
  Creates new UI components and ECS features for the codelord editor following
  established patterns. Use when user says "create component", "add feature",
  "new panel", "new page", "new view", or "scaffold". Walks through ECS
  module creation in codelord-core and UI component in codelord-components.
---

# Codelord Component Creator

Scaffold a new feature with proper ECS + UI structure.

## Instructions

### Step 1: Determine component type

Ask the user (if not clear) what kind of component:
- **Page**: Full editor view (text editor, terminal, settings)
- **Panel**: Side/bottom panel (explorer, search, music player)
- **View**: Content area within a page (copilord chat, editor content)
- **Overlay**: Floating UI (file picker, popup, toast)
- **Atom**: Small reusable widget (button, icon button)
- **Organism**: Complex composed section (titlebar, statusbar)

### Step 2: Create ECS module in codelord-core

Create `codelord-core/src/{feature_name}/` with:

1. `mod.rs` - Module declaration and re-exports
2. `components.rs` - ECS component structs (plain data, no logic)
3. `resources.rs` - ECS resource structs (shared state)
4. `systems.rs` - System functions (all logic lives here)

CRITICAL: Components are pure data. Resources hold state. Systems contain logic.

### Step 3: Create UI component in codelord-components

Create the appropriate file in `codelord-components/src/components/{type}/`.

The UI component:
- Takes `&mut egui::Ui` and relevant Resources as parameters
- Renders UI using egui
- Returns user actions/responses back to the caller
- Does NOT mutate state directly

### Step 4: Register in module trees

1. Add `pub mod {feature_name};` to `codelord-core/src/lib.rs`
2. Add the UI component to the appropriate module in `codelord-components/src/components/{type}.rs`
3. Wire the component into the relevant page/panel layout

### Step 5: Verify

- Run `just clippy` to check for warnings
- Run `just build` to verify compilation
- Confirm ECS pattern compliance (no logic in components, no state in UI)

## Example: Creating a "minimap" panel

```
codelord-core/src/minimap/
  mod.rs            # pub mod components; pub mod resources; pub mod systems;
  components.rs     # pub struct MinimapVisible(pub bool);
  resources.rs      # pub struct MinimapState { pub scroll_ratio: f32, ... }
  systems.rs        # pub fn update_minimap(state: &mut MinimapState, ...) { ... }

codelord-components/src/components/panels/
  minimap.rs        # pub fn show(ui: &mut Ui, state: &MinimapState) -> MinimapResponse
```
