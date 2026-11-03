# Rust Game Development Guide

Setup and migration reference for Rust (Edition 2021), Macroquad + toolkit,
Windows and WebGL/WASM. Begin with [AGENTS.md](AGENTS.md); coding policy lives
in [CODE_STANDARDS.md](CODE_STANDARDS.md), screen design in [UI_STYLE.md](UI_STYLE.md),
and API examples in [MACROQUAD_TOOLKIT.md](MACROQUAD_TOOLKIT.md).

## Quick Start

### New Game Setup

Start from the working template, not `cargo new`. From the workspace root:

```powershell
Copy-Item .\rust_management\template .\my_game -Recurse
# Follow the copied README's "Rename For A New Game" checklist.
# Add my_game to rust_management/workspace/Cargo.toml members, then:
python .\rust_management\sync-workspace.py
Set-Location .\my_game
```

The template README is the single rename checklist: package/data, toolkit path,
asset registry, page metadata, thumbnail, capture settings, and optional itch
configuration. Initialize the game's independent Git repository and intended
remote. Record the UI screen brief in its GDD/README and recompose the demo
before expanding content. Commit each validated buildable feature slice per
[AGENTS.md](AGENTS.md#commits), not just the finished game.

### Dependencies (`Cargo.toml`)

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2021"

[dependencies]
macroquad = "=0.4.16"
macroquad-toolkit = { path = "../macroquad-toolkit" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

The nested template uses `../../macroquad-toolkit`; change it after copying to
a workspace-root sibling. Use `..\rust_management\cargo.ps1` for compilation,
checks/tests/Clippy/run. See `rust_management/docs/CARGO_WORKSPACE.md` for the
three-slot pool, exact dependency pin, editor routing, and canonical root config.

## Architecture Overview

### Web to Rust Migration Map

| Web concern | Rust equivalent |
| --- | --- |
| React/DOM/CSS | Macroquad canvas and toolkit immediate-mode UI |
| PHP/Node logic | Rust services; keep server authority for multiplayer |
| MySQL | JSON content or native/server database as appropriate |
| CSS styling | Toolkit styles/layout with data-driven game configuration |

### Tech Stack Philosophy

Macroquad is the thin rendering/input/audio/timing layer. Game structs own state
and explicit transitions; toolkit widgets provide UI. See code standards
sections 2, 5, and 7 for boundaries, named modules, and UI intents.

### Client/server games

- Client renders the server's projection and sends intent-like commands.
- Server owns validation, simulation time, world state, and durable persistence.
- A small protocol crate owns shared wire types. Client-local storage holds UI
  preferences and credentials unless offline play is deliberately designed.
- Enable toolkit `features = ["net"]` for non-blocking native/WASM JSON HTTP
  through `HttpClient` and `Pending<T>`; the publisher supplies `quad-net.js`.
  Games own endpoints, protocol, handshake/reconnect, authority, CORS,
  authentication verification, and database schema. Read the toolkit's net
  section for frame polling, timeouts, safe failure state, and retry cooldowns.

## Project Structure

Use [CODE_STANDARDS.md section 2](CODE_STANDARDS.md#2-project-structure-rules)
and the working template. Do not introduce new `mod.rs` files; migrate legacy
module roots to named files when restructuring, never leave both forms.

## Core Patterns

### Entry Point (`main.rs`)

A single `#[macroquad::main(window_conf)]` loop owns `Game`, calls update then
draw, and awaits `next_frame()`. Configure the resizable window explicitly;
the template also wires deterministic screenshot capture.

### State Machine Pattern

Use `GameState` variants carrying each phase's state and explicit
`StateTransition` values. One state is active; no shared mutable globals.

### Individual State Pattern

Each state updates its data and returns `Option<StateTransition>`; `None` stays
in the current phase. Its draw method reads state only.

### Game Struct (`game.rs`)

`Game` owns the active state and shared assets, dispatches update/draw, and
applies transitions. Extract cohesive modules as responsibilities grow.

## UI: Immediate Mode

### Layout (Replacing CSS Flexbox)

Use toolkit layout, virtual UI, camera, and pointer helpers. Keep layout and
pointer coordinates logical; convert only at the physical framebuffer viewport
boundary. Examples are in the toolkit reference and working starter.

### UI Philosophy

Views return intents for game logic to apply. Use release-triggered toolkit
buttons by default, not copied mouse-only widgets. Follow the complete
[UI_STYLE.md](UI_STYLE.md) brief, subtraction pass, and visual review.

## Data Loading

### JSON Definition (`assets/cards.json`)

Content, balance, configuration, and player-facing text belong in `assets/`
JSON. Games own typed schemas and semantic validation (IDs/references/rules).

### Loader (`data/loader.rs`)

Use `macroquad_toolkit::include_json!` for embedded data or typed
`data_loader` functions for runtime/native loading. Generic parsing, diagnostics,
platform handling, and fallback belong to the toolkit; see its JSON examples.

## Persistence (Save/Load)

### JSON (Recommended for Save Files)

Keep a versioned, Serde-friendly save type. Use toolkit persistence slot APIs;
the template demonstrates versioned saves, migrations, listing, and deletion.
Handle unsupported/old saves with a clear recoverable error, never a crash;
let the player start fresh. Backward compatibility and migrations are optional
for demos unless requested. Versioned/migration APIs are available, not mandates.

### Native/Server Databases

Database crates belong only in native/server code. WebGL clients use JSON and
toolkit persistence, not browser-incompatible filesystem/database access.

## Deployment

### Required Files

Keep `publish.ps1`, `game_page.json`, and root `catalog_thumbnail.png`;
[code standards section 8](CODE_STANDARDS.md#8-deployment--web-standards)
is the deployment and validation authority.

### Validation

Follow [section 8.3](CODE_STANDARDS.md#83-validation): formatting, strict Clippy,
source-size gates, focused existing tests, and review of changed UI only. Write
no new tests by default ([section 11](CODE_STANDARDS.md#11-testing-policy)). Broaden
checks for cross-cutting risk or integration/release acceptance. Publishing is
separate, needs user authorization, and never gates a local slice commit.

### Build Targets

Use the shared launcher for Windows release and `--target wasm32-unknown-unknown`
release builds. The parameterless publisher builds/packages both targets.

### Generated Web Page (`game_page.json`)

Only `title` is required; defaults derive from the project directory. Supply the
WASM/package name, player-facing copy, touch controls, and canvas behavior there.
The publisher combines it with `rust_management/web/index.template.html` into
`dist/webgl/index.html`. No hand-maintained game-root HTML or per-game CSS/JS;
see `rust_management/web/README.md` for schema and shared runtime behavior.

### Catalog Thumbnail

Keep a 16:9 title/menu PNG at `catalog_thumbnail.png`, deployed to
`<game_slug>/catalog_thumbnail.png`. The catalog uses a title-banner fallback
until a capture exists. `capture-title-screenshots.ps1 -Publish` refreshes title
captures across games; use only for an authorized catalog-wide refresh.

## Future Image Prompts

### Catalog (`assets/image_prompts.json`)

Keep placeholder-to-generated-image plans in JSON: each asset ID records
`prompt`, `filename`, `width`, and `height`. Dimensions must be divisible by 16.
Define prompts, develop with missing-file placeholders, generate images, then
place them in `assets/` and register runtime-loaded paths for packaging.

## Checklists

### New Game

Follow the copied template README rename checklist and complete the code/UI
validation references above. Do not maintain another duplicate setup checklist.

### Migration (Web → Rust)

Define Rust entities and the main/state loop; copy template publishing metadata;
port backend logic while preserving any required server authority; migrate data
to JSON or a native/server database; redesign screens around player decisions;
wire intents and persistence; validate both platforms and touch UI. Commit each
coherent validated slice throughout the migration.

## Non-Goals

Avoid ECS overengineering and custom editor tooling initially. Stabilize core
play before procedural generation; prefer the simplest maintainable design.
