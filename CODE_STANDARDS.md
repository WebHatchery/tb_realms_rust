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
Keep game content and configuration in JSON; see §5.3 for loading and validation rules.

### 1.4 No Unused Code
Delete unused variables, fields, and functions; never hide them with `_` prefixes. An unused parameter may use an `_` prefix only when a required trait or API signature prevents removing it.

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
- Data types have no knowledge of engine or UI; all domains may read them.
- See §5.1 for state ownership and §7 for UI actions.

### 2.2 File Size Guideline
- Target 200–400 lines per file; begin planning a split at 600.
- Every `.rs` file has a hard limit of 800 total physical lines, including whitespace, comments, attributes, and tests. Implementation, test, generated source, example, build-script, and bench files have no exemptions.
- Extract a cohesive responsibility before a change exceeds the limit. Restructure any existing oversized file encountered during the task before completing it.
- Never meet the limit by stripping spacing, compressing formatting, or moving a single small function solely to reduce the count.
- Use the source gate described in `MACROQUAD_TOOLKIT.md`; exception lists must be empty.

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
├── UI_STYLE.md             # Screen composition and visual review
├── src/
│   ├── lib.rs              # Public game logic used by the binary and tests
│   ├── main.rs             # Entry point and game loop
│   ├── data.rs             # Data module root and re-exports
│   ├── data/               # Data child modules
│   │   ├── schema.rs       # Typed schemas and game-specific validation
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
├── tests/                  # This crate's tests and test-only helpers
│   └── gameplay.rs         # Tests through the public library API
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
- `GameState` owns current state; `PlayerStats` owns persistent progression.
- `Game` coordinates state mutations through explicit actions and transitions; action handlers may live in dedicated modules.
- Engine services receive state and return results rather than owning mutable state.
- UI reads state and returns intents for the dispatcher to apply (§7).

### 5.2 Prefer Plain Data
Use structs with clear fields. Avoid overly clever enums with embedded logic unless they model a real state machine.  

Game data should be:  
- Serializable (Serde-friendly for save/load)  
- Easy to debug and inspect  
- Immutable after loading from JSON  

### 5.3 Data-Driven Design
- Store game constants, balance, configuration, content, and player-facing text as JSON under `assets/`; reference loaded values rather than hardcoding them.
- Load at startup through `macroquad_toolkit::include_json!` for embedded data or typed `macroquad_toolkit::data_loader` functions for runtime/native loading.
- Projects own Serde-backed schemas and semantic validation: IDs, references, balance invariants, and game rules.
- The toolkit owns generic parsing, file loading, platform branching, source-labeled diagnostics, and fallback behavior. Do not create generic project-local loader wrappers or call `serde_json::from_str` directly for game-data files.

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

Read [UI_STYLE.md](UI_STYLE.md) before designing or changing a screen. It defines
the required screen brief, visual hierarchy, progressive disclosure, template
adaptation, and visual review. The rules below govern implementation.

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
Use shared toolkit widgets, input helpers, and palettes. Prefer buttons that fire on release; use press actions only when immediate feedback is intentional. See `MACROQUAD_TOOLKIT.md` for imports, API examples, and button semantics.

### 7.5 Browser Controls and Layout
- Games are touch-first: starting, tutorials, core interactions, and recovery must work through visible tap/click controls without a physical keyboard.
- Keyboard shortcuts may supplement controls. Player-facing shortcut text must also name the equivalent visible touch control.
- Tutorial prompts name the exact visible control or gesture needed next, such as “Tap CONTINUE” or “Drag the map.”
- Dismiss completed tutorial prompts and keep help reopenable; preserve visible, understandable controls during normal play (`UI_STYLE.md` §5).
- Keep drawing separate from mutation. Support common desktop browser sizes and responsive scaling; use fixed positions only with an intentional virtual resolution.

## 8. Deployment & Web Standards

### 8.1 Required Files
Every game must have these files for deployment:
- `publish.ps1` – Build and deploy script
- `game_page.json` – Per-game metadata used to generate the WebGL host page
- `catalog_thumbnail.png` – Root-level catalog image

### 8.2 Build Targets
Use the shared `..\rust_management\cargo.ps1` launcher for concurrent local builds, checks, tests and Clippy. It leases a bounded shared build slot and uses sccache when installed; it does not copy source or change workspace membership. Publishing and capture integrate it automatically. See `rust_management/docs/CARGO_WORKSPACE.md`.

The game must build for:
- **Windows**: `..\rust_management\cargo.ps1 build --release`
- **Web/WASM**: `..\rust_management\cargo.ps1 build --release --target wasm32-unknown-unknown`

### 8.3 Validation
After meaningful game changes, run `.\publish.ps1` with no parameters from the affected project directory and report the result. If the script is missing, blocked, or fails for an unrelated environment reason, report that limitation. A local instance or dev server is not a substitute unless the user requests it.

All validation must run against the actual project checkout being changed, with
its real workspace and dependency configuration. Do not create or use isolated
project copies, copied source trees, temporary clones, alternate manifests, or
fabricated workspaces to get formatting, Clippy, source-size gates, tests, or
publishing to pass. Do not detach a game from its workspace or change dependency
paths merely to bypass a validation failure.

For example, "Validation passed in the isolated project copy: formatting,
clippy with `-D warnings`, source-size gate, and 15 gameplay tests" is not an
acceptable substitute for validating the changed checkout. Even an honestly
labelled isolated-copy result does not satisfy these requirements. Run the
checks in the actual checkout; if its workspace cannot load, report that error
and identify the blocked checks. Fix the real cause within the task's scope,
then rerun validation there. Never claim completion based on a copied project.

Use project-local asset paths and make missing assets and loading failures clear during publishing.

### 8.4 WebGL Requirements
The publisher generates `dist/webgl/index.html` from
`rust_management/web/index.template.html` and the game's `game_page.json`.
Do not maintain a project-root `index.html` for a migrated game. Configure the
title, WASM name, controls, page copy, canvas behavior, and Project Roost slug
in `game_page.json`; change the shared template only for catalog-wide behavior.

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

### 11.3 Feature Test Target
- Strongly target no more than five `#[test]` cases per major feature: one cohesive responsibility, regardless of how its tests are split or named.
- Prefer high-value behavior and regression tests. Consolidate related inputs with table-driven assertions; do not bundle unrelated checks or delete useful coverage to meet the target.
- Before committing, review affected feature suites. If more than five cases are needed, briefly explain why distinct coverage warrants them.

### 11.4 Test Placement
- Each crate owns a `tests/` directory beside its `Cargo.toml`, including member crates in multi-crate repositories. Keep all tests and test-only helpers there.
- Do not add `#[cfg(test)]`, `mod tests`, test helpers, or test source files under `src/`.
- Tests exercise the crate's public API. For a binary-only game, expose testable logic through `src/lib.rs` and have `main.rs` use that library; keep internals private unless an intentional public seam is needed.
- Existing `src/**/tests.rs` files are legacy migration work. Migrate them as a separate change before expanding coverage.
- Split large suites by responsibility while preserving the feature target (§11.3) and file-size rule (§2.2).

## 12. Verification Artifacts

- For UI changes, follow the visual and interaction review in `UI_STYLE.md` §9; inspect normal and minimum supported sizes and relevant dense states. Compilation and geometry checks alone do not verify usability.
- Store verification screenshots directly in `docs/verification/`.
- Do not create screenshot subfolders under `docs/verification/`.
- If a new capture represents the same screen or state as an existing screenshot, replace the existing image instead of keeping duplicates.
- Do not create disposable review files, scratch projects, backup screenshots, or cleanup folders, either inside the workspace or elsewhere. Do not move files out of a repository to make Git status clean. Preserve existing work and report blockers instead.
- Use existing tooling and direct command output. Established tools may manage their own internal capture manifests, logs, and normal build outputs; do not create an ad hoc parallel set of temporary artifacts.
- Never fabricate a `Cargo.toml`, source file, or placeholder crate to bypass a workspace failure. A missing manifest in an unexpected directory is a workspace hygiene problem to investigate and report, not a request to invent a project.

Correct capture workflow (from the game directory):

```powershell
# Capture required evidence directly to its stable, documented location.
# Use scene names supported by this game; the wrapper is hidden by default.
& ..\macroquad-toolkit\scripts\capture_ui.ps1 -Scenes gameplay -OutputDir docs\verification
# Wait for completion and check the exit result before inspecting the image.
if (-not $?) { throw 'Capture failed; investigate the reported error.' }
# Re-capture to the same filename when iterating; do not create backup copies.
# If workspace discovery fails, inspect/report the offending directory.
# Never create a dummy Cargo.toml or move files into a sibling cleanup folder.
```
