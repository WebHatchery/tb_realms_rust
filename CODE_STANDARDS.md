# Rust Coding Standards for Macroquad Games

**Engine**: Macroquad + macroquad-toolkit  
**Language**: Rust  
**Platform**: WebGL (WASM) + Native

This document defines the centrally maintained coding standards for Macroquad game projects. Keep project-local copies identical to the canonical `docs/CODE_STANDARDS.md`; put project-specific guidance in the project's README or another local documentation file instead.

These standards prioritize:  
- Readability over cleverness  
- Data-driven design over hardcoded values  
- Clean state management  
- Modular services for game logic  
- A clear mental model for game phases and transitions  

## 1. Core Philosophy

### 1.1 Write for Maintainability
Code should be easy to debug and extend.  
- Prefer obvious, straightforward code  
- Avoid hidden state or side effects  
- If a junior Rust developer can understand the flow, you are doing it right.

### 1.2 Consistency Beats Preference
If a pattern already exists in the codebase, follow it even if you dislike it. A consistent codebase is more valuable than a perfect one.

### 1.3 Data-Driven Design
All game constants, balance values, and static data should be defined in JSON files under `assets/`. Load this data at startup using Serde for easy balancing and iteration without recompiling code. Avoid hardcoding values in Rust code; reference loaded data structures instead.

### 1.4 No Unused Code
- Remove unused variables, fields, and functions immediately
- Never suppress unused warnings with `_` prefixes on struct fields
- If a field is unused, delete it - don't mark it as unused
- Parameter prefixes with `_` are acceptable only when required by trait signatures

## 2. Project Structure Rules

### 2.1 Module Responsibilities
Each module/subdirectory owns a single conceptual domain:

**Root Level:**
- `main.rs` – Entry point, game loop, phase transitions, and high-level coordination

**Subdirectories:**
- `data/` – Data structures and JSON loading
  - Type definitions for game entities
  - Constants and configuration structures

- `engine/` – Game logic services (stateless where possible)
  - Core game calculations
  - Entity management and state machines
  - Visual effects (particles, transitions)

- `state/` – Game state management
  - Current game state
  - Persistent player progression
  - Save/load functionality

- `ui/` – User interface components
  - Base UI utilities and styling
  - Reusable UI widgets
  - Uses macroquad-toolkit for buttons and interactions

- `screens/` – Screen-specific rendering (if separated from main.rs)

**Cross-Domain Rules:**
- ❌ UI must never mutate game state directly
- ❌ Engine services should be stateless - receive state, return results
- ❌ Data module has no knowledge of engine or UI
- ✅ All domains can read from `data/` types
- ✅ State mutations happen only in main.rs via clearly defined actions

### 2.2 File Size Guideline
- Target: 200–400 lines per file
- Soft limit: 600 lines
- Hard limit: 800 lines for every `.rs` file
- If a file grows beyond this, split by responsibility.
- **Limits count non-test lines only.** A `#[cfg(test)] mod tests` block does not count toward the target, the soft limit, or the hard limit. The limit exists to catch files that have taken on too many responsibilities; a test module is one cohesive block you scroll past once, and it is compiled out of release and WASM builds entirely. See §11.3 for when to move it to its own file anyway.

### 2.3 Module Source Filenames
- Use Rust's named module source filenames: `foo.rs` for `mod foo;`, and `foo/bar.rs` for `mod bar;` inside `foo.rs`.
- Do not create new `mod.rs` files.
- When restructuring existing modules, prefer migrating `foo/mod.rs` to `foo.rs` and keeping child modules under `foo/`.
- Do not keep both `foo.rs` and `foo/mod.rs`; Rust treats that as an ambiguous module source.

### 2.4 Folder Structure

```
game_name/
├── Cargo.toml              # Project manifest
├── CODE_STANDARDS.md       # This file
├── src/
│   ├── main.rs             # Entry point, game loop, screen rendering
│   ├── data.rs             # Data module root and re-exports
│   ├── data/               # Data child modules
│   │   ├── loader.rs       # JSON deserialization
│   │   └── constants.rs    # Game constants structures
│   ├── engine.rs           # Engine module root and re-exports
│   ├── engine/             # Engine child modules
│   │   └── game_engine.rs  # Core calculations
│   ├── state.rs            # State module root and re-exports
│   ├── state/              # State child modules
│   │   ├── game_state.rs   # Current game state
│   │   └── persistence.rs  # Save/load
│   ├── ui.rs               # UI module root and re-exports
│   ├── ui/                 # UI child modules
│   │   ├── core.rs
│   │   └── components.rs
│   └── screens.rs          # Screen renderers module root (optional)
├── assets/                 # Game data
│   ├── constants.json      # Balance values
│   └── localization/       # Text strings
└── .gitignore
```

## 3. Naming Conventions

### 3.1 General Rules
- Types: PascalCase  
- Functions & variables: snake_case  
- Constants: SCREAMING_SNAKE_CASE  
- Modules: snake_case  

Names should describe what the thing is, not how it works.

### 3.2 Boolean Naming
Booleans should read like facts:  
```rust
is_active  
can_interact  
has_unlocked  
should_update  
```  
Avoid `flag`, `value`, or `state` in names.

### 3.3 Service Naming
Engine services follow a naming pattern:
- `*Service` for stateless helpers
- `*Engine` for complex stateless processors
- `*StateMachine` for state progressions

## 4. Functions & Methods

### 4.1 Function Size
- Target: 20–50 lines  
- Absolute max: 100 lines  
- If a function needs scrolling, it probably needs refactoring.

### 4.2 Single Responsibility
Each function should answer one question or perform one action.

### 4.3 Argument Count
- Prefer ≤ 3 parameters  
- If more are needed, use a struct or reference to state  
- Services should take `&GameState` or `&Config` rather than many individual fields

### 4.4 Return Types
- Use `Option<T>` for potentially missing values  
- Use custom result structs for complex outcomes
- Avoid returning multiple values via tuple; create a named struct instead

## 5. Data & State Management

### 5.1 Game State Ownership
- `GameState` owns the current game state  
- `PlayerStats` owns persistent progression  
- Mutation happens through methods on `Game` struct in main.rs  
- Services return results; they don't mutate state directly  

### 5.2 Prefer Plain Data
Use structs with clear fields. Avoid overly clever enums with embedded logic unless they model a real state machine.  

Game data should be:  
- Serializable (Serde-friendly for save/load)  
- Easy to debug and inspect  
- Immutable after loading from JSON  

### 5.3 Data-Driven Design
- All game balance and configuration in JSON under `assets/`
- Load data at application startup; data is embedded at compile time
- Use structs that mirror JSON structure for type safety
- Never hardcode magic numbers; reference loaded config data

### 5.4 Enums for Game Phases
Use enums to model distinct game states:
```rust
pub enum GamePhase {
    Loading,
    MainMenu,
    Playing,
    Paused,
    GameOver,
    // Add game-specific phases
}
```

## 6. Error Handling

### 6.1 Prefer Option Over Panics
- `panic!` is acceptable only for truly unrecoverable states  
- Missing entities or items should return `None`, not panic  
- Use:  
  - `Option<T>` for potentially missing values  
  - `Result<T, E>` for fallible I/O operations (save/load)  
  - Graceful degradation for missing data  

### 6.2 Logging Over Silent Failures
Use `eprintln!` for error conditions that should be visible during development but shouldn't crash the game.

## 7. UI Code (Macroquad-Toolkit)

### 7.1 UI Is Dumb
UI code:  
- Reads game state  
- Returns actions/intents  
- It should never contain game logic.  

### 7.2 Action Pattern
UI components return `Option<UiAction>` to signal user intent:
```rust
pub enum UiAction {
    StartGame,
    Pause,
    Resume,
    // Add game-specific actions
}
```

### 7.3 Component Organization
- `core.rs` – Color schemes, fonts, base styling  
- `components.rs` – Reusable widgets  
- Each component is a pure function: `fn draw_thing(state: &State) -> Option<UiAction>`

### 7.4 Macroquad-Toolkit Usage

Use `macroquad-toolkit` for common UI patterns. Prefer `use macroquad_toolkit::prelude::*;` for common helpers, or explicit `macroquad_toolkit::ui::*` imports.

**Available Modules:**
- `macroquad_toolkit::ui::button()` - Standard clickable button (fires on release)
- `macroquad_toolkit::ui::button_on_press()` - Button that fires on mouse down
- `macroquad_toolkit::ui::button_styled()` - Button with custom styling
- `macroquad_toolkit::ui::panel()` - Draws a panel with optional title
- `macroquad_toolkit::ui::progress_bar()` - Progress indicator
- `macroquad_toolkit::colors::dark::*` - Standard dark theme colors
- `macroquad_toolkit::input::*` - Mouse/keyboard input helpers

**Button Click Semantics:**
```rust
// Standard button - fires on mouse RELEASE (safer, allows cancel)
if button(x, y, w, h, "Click Me") {
    return UiAction::DoThing;
}

// Press button - fires on mouse DOWN (instant feedback)
if button_on_press(x, y, w, h, "Emergency", &style) {
    // Immediate action
}
```

**Color Palette:**
```rust
use macroquad_toolkit::colors::dark;

clear_background(dark::BACKGROUND);  // Standard background
draw_rectangle(x, y, w, h, dark::PANEL);  // Panel color
draw_text("Hello", x, y, 20.0, dark::TEXT);  // Text color
// Also: dark::ACCENT, dark::POSITIVE, dark::WARNING, dark::NEGATIVE
```

**Input Helpers:**
```rust
use macroquad_toolkit::input::*;

if is_hovered(x, y, w, h) { /* Mouse over area */ }
if was_clicked(x, y, w, h) { /* Left click released on area */ }
if was_pressed(x, y, w, h) { /* Left click pressed on area */ }
```

## 8. Deployment & Web Standards

### 8.1 Required Files
Every game must have these files for deployment:
- `publish.ps1` – Build and deploy script
- `index.html` – WebGL host page

### 8.2 Build Targets
The game must build for:
- **Windows**: `cargo build --release`
- **Web/WASM**: `cargo build --release --target wasm32-unknown-unknown`

### 8.3 Validation
After meaningful changes, run `.\publish.ps1` with no parameters from the affected project directory.

### 8.4 WebGL Requirements
The `index.html` must:
- Load `mq_js_bundle.js` (Miniquad loader)
- Call `load("game_name.wasm")`
- Include canvas with `id="glcanvas"`
- Use `image-rendering: pixelated` for pixel art

### 8.5 Catalog Thumbnail
Each published game should keep `catalog_thumbnail.png` in the project root. Use a 16:9 title-screen or main-menu capture. The shared publisher deploys the file as `<game_slug>/catalog_thumbnail.png`, and the WebHatchery games catalog uses that stable path for card thumbnails.

## 9. Comments & Documentation

### 9.1 Comment Why, Not What
Code already explains what it does. Comments should explain why it exists.

### 9.2 Module-Level Docs
Each module should contain a short `//!` comment explaining its purpose:
```rust
//! Player inventory and item effects.
```

## 10. Formatting & Tooling

### 10.1 rustfmt
- Always use `cargo fmt`  
- Never fight the formatter  

### 10.2 Clippy
- Run `cargo clippy` regularly  
- Fix warnings unless intentionally ignored  
- Document any `#[allow]` with a comment

### 10.3 Variable Shadowing
- Avoid variable shadowing (hiding)
- Do not declare a new variable with the same name as an existing one in the same scope

### 10.4 Unused Code
- Remove unused variables immediately
- Remove unused struct fields immediately  
- Never use `_` prefix on struct fields to suppress warnings
- `_` prefix on function parameters is acceptable when required by API

## 11. Testing Guidelines

### 11.1 What to Test
Focus tests on:  
- Core game calculations  
- State machine transitions  
- JSON data loading  
- UI and rendering generally do not need unit tests.

### 11.2 Test Style
- Tests should read like rules  
- Avoid complex setups  
- If a test is hard to write, the code is probably too tangled.

### 11.3 Test Placement
Unit tests live in the crate, next to the code they cover — as an inline `#[cfg(test)] mod tests` block at the bottom of the file. This is the Rust convention, it keeps `use super::*` access to private items, and it adds no compile units.

**Do not move unit tests to a `tests/` directory.** Files under `tests/` are integration tests: each one compiles and links as a separate crate, and it can only reach the crate's *public* API. Most games here are binary crates with no `lib.rs`, so `tests/` cannot import them at all, and widening visibility purely to satisfy a test layout works against §2.1's cross-domain rules. Reserve `tests/` for genuine end-to-end or balance harnesses that exercise a lib target's public surface.

**When a test module dominates its file, extract it to a child module** — not to `tests/`:

```rust
// src/simulation.rs
#[cfg(test)]
mod tests;          // -> src/simulation/tests.rs
```

This keeps `use super::*` and same-crate access while getting the bulk out of the parent file. It follows the named-module rule in §2.3, so use `foo/tests.rs`, never `foo/tests/mod.rs`.

Extract when either holds:
- the test module exceeds ~300 lines, or
- it is more than half the file

Below that, leave tests inline. A 40-line test block under a 200-line module is idiomatic and moving it costs a file and an indirection for no gain.

## 12. Final Rule

If a piece of code feels fragile, confusing, or brittle, it probably is. Refactor early. Leave the code calmer than you found it.
