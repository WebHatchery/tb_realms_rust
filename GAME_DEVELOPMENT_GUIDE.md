# Rust Game Development Guide

**Engine**: Macroquad + macroquad-toolkit  
**Language**: Rust (Edition 2021)  
**Platform**: WebGL (WASM) + Native Windows

This guide covers both creating new games and migrating existing web applications to standalone Rust games.

---

## Quick Start

### New Game Setup

```bash
cargo new my_game
cd my_game
```

### Dependencies (`Cargo.toml`)

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2021"

[dependencies]
macroquad = "0.4"
macroquad-toolkit = { path = "../macroquad-toolkit" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

> **Note**: Profile settings (`[profile.release]`) are defined at the workspace root.

---

## Architecture Overview

### Web to Rust Migration Map

| Feature | Web (Old) | Rust (New) |
| :--- | :--- | :--- |
| **Frontend** | React/DOM/CSS | Macroquad (Canvas, Immediate UI) |
| **Backend** | PHP/Node | Rust internal logic |
| **Database** | MySQL | JSON data or native/server DB |
| **Styling** | CSS | Rust constants/functions |

### Tech Stack Philosophy

**Use Macroquad for:**
- Rendering (shapes, textures, text)
- Input handling (keyboard, mouse)
- Audio playback
- Main loop timing

**Do NOT use Macroquad for:**
- Scene management (use state machines)
- Game state authority (use your own structs)
- UI framework (use immediate-mode from macroquad-toolkit)

> Macroquad should remain a *thin* rendering/input layer.

---

## Project Structure

```
game_name/
├── Cargo.toml
├── CODE_STANDARDS.md       # Coding standards
├── publish.ps1             # Build & deploy script
├── index.html              # WebGL host page
├── src/
│   ├── main.rs             # Entry point, window config
│   ├── game.rs             # Game loop & state machine
│   ├── state.rs            # State module root and re-exports
│   ├── state/              # State child modules
│   │   ├── menu.rs
│   │   └── gameplay.rs
│   ├── engine.rs           # Engine module root and re-exports
│   ├── engine/             # Engine child modules
│   │   └── game_engine.rs
│   ├── data.rs             # Data module root and re-exports
│   ├── data/               # Data child modules
│   │   └── loader.rs
│   ├── ui.rs               # UI helpers module root
│   └── save.rs             # Persistence
├── assets/
│   ├── data.json           # Game data
│   └── images/             # Sprites
└── README.md
```

Use Rust's named module source filenames: `foo.rs` for `mod foo;`, and `foo/bar.rs` for child modules declared inside `foo.rs`. Do not create new `mod.rs` files; when restructuring old modules, migrate `foo/mod.rs` to `foo.rs`.

---

## Core Patterns

### Entry Point (`main.rs`)

```rust
use macroquad::prelude::*;

mod game;
mod state;
mod data;

use game::Game;

fn window_conf() -> Conf {
    Conf {
        window_title: "Game Name".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;
    
    loop {
        clear_background(Color::from_rgba(20, 20, 25, 255));
        game.update();
        game.draw();
        next_frame().await;
    }
}
```

### State Machine Pattern

```rust
// state.rs
pub enum GameState {
    Menu(MenuState),
    Gameplay(GameplayState),
    Results(ResultState),
}

pub enum StateTransition {
    ToMenu,
    ToGameplay(GameplayState),
    ToResults(ResultState),
}
```

**Rules:**
- Only ONE state active at a time
- Transitions are explicit (no magic callbacks)
- No shared mutable global state

### Individual State Pattern

```rust
pub struct GameplayState {
    // State-specific data
}

impl GameplayState {
    pub fn new() -> Self { ... }
    
    pub fn update(&mut self) -> Option<StateTransition> {
        // Return None to stay, Some(transition) to change
    }
    
    pub fn draw(&self, textures: &HashMap<String, Texture2D>) {
        // Render this state
    }
}
```

### Game Struct (`game.rs`)

```rust
pub struct Game {
    pub state: GameState,
    pub textures: HashMap<String, Texture2D>,
}

impl Game {
    pub async fn new() -> Self { ... }
    
    pub fn update(&mut self) {
        // Match on current state, call state.update()
        // Handle StateTransition return values
    }
    
    pub fn draw(&self) {
        // Match on current state, call state.draw()
    }
    
    pub fn transition(&mut self, transition: StateTransition) {
        // Apply explicit state change
    }
}
```

---

## UI: Immediate Mode

### Layout (Replacing CSS Flexbox)

**React (CSS):**
```css
.container { display: flex; justify-content: center; }
```

**Rust:**
```rust
let center_x = screen_width() / 2.0;
let button_w = 200.0;
let start_x = center_x - button_w / 2.0;
let mut y = 100.0;
const PADDING: f32 = 20.0;

if button(start_x, y, button_w, 50.0, "Start Game") {
    // Handle click
}
y += 50.0 + PADDING;
```

### UI Philosophy

```rust
fn draw_button(x: f32, y: f32, text: &str) -> bool {
    let rect = Rect::new(x, y, 200.0, 40.0);
    let hovered = rect.contains(mouse_position().into());
    let clicked = hovered && is_mouse_button_pressed(MouseButton::Left);
    
    let color = if hovered { LIGHTGRAY } else { GRAY };
    draw_rectangle(x, y, 200.0, 40.0, color);
    draw_text(text, x + 10.0, y + 28.0, 24.0, WHITE);
    
    clicked
}
```

**Rules:**
- UI reads state, returns intents (bools/enums)
- UI never contains game logic
- Game logic applies changes

---

## Data Loading

### JSON Definition (`assets/cards.json`)

```json
[
  {
    "id": "strike",
    "name": "Strike",
    "cost": 1,
    "description": "Deal 6 damage",
    "effects": [{ "Damage": 6 }]
  }
]
```

### Loader (`data/loader.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardData {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub description: String,
    pub effects: Vec<CardEffect>,
}

impl CardData {
    pub async fn load_all() -> Result<Vec<CardData>, String> {
        macroquad_toolkit::data_loader::load_json_file("assets/cards.json").await
    }
}
```

---

## Persistence (Save/Load)

### JSON (Recommended for Save Files)

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub progress: ProgressData,
}

impl SaveData {
    pub fn save(&self) -> Result<(), String> {
        macroquad_toolkit::persistence::save_to_slot("my_game", "slot_1", self)
    }
    
    pub fn load() -> Result<Self, String> {
        macroquad_toolkit::persistence::load_from_slot("my_game", "slot_1")
    }
}
```

### Native/Server Databases

Use database crates only for native/server code. Keep WebGL clients on JSON data plus toolkit persistence.

---

## Deployment

### Required Files

Every game MUST have:
- `publish.ps1` – Build and deploy script
- `index.html` – WebGL host page
- `catalog_thumbnail.png` – Root-level catalog thumbnail, preferably a title-screen capture. The publisher deploys this as `<game_slug>/catalog_thumbnail.png`.

### Validation

Run this with no parameters from the affected project directory after meaningful changes:

```powershell
.\publish.ps1
```

### Build Targets

```bash
# Windows release
cargo build --release

# WebGL/WASM
cargo build --release --target wasm32-unknown-unknown
```

### Web Template (`index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>My Game | Web Hatchery</title>
    <link rel="stylesheet" href="../shared.css">
    <style>
        .game-container canvas {
            image-rendering: pixelated;
        }
    </style>
</head>
<body>
    <div class="game-page">
        <canvas id="glcanvas" tabindex="1"></canvas>
    </div>
    <script src="mq_js_bundle.js"></script>
    <script>load("my_game.wasm");</script>
</body>
</html>
```

### Catalog Thumbnail

Use `catalog_thumbnail.png` in the project root for the WebHatchery games catalog card. The file should be a 16:9 PNG from the game's title or main menu screen. If a title screen has not been captured yet, the catalog falls back to a simple title banner until the file exists.

The root publisher looks for this exact filename and deploys it to:

```text
<game_slug>/catalog_thumbnail.png
```

To refresh title captures from preview builds and copy them into project roots:

```powershell
.\capture-title-screenshots.ps1 -Publish
```

---

## Future Image Prompts

Use a JSON catalog for managing placeholder-to-generated-image transitions.

### Catalog (`assets/image_prompts.json`)

```json
{
  "player_idle": {
    "prompt": "A futuristic space marine standing idle, pixel art style",
    "filename": "player_idle.png",
    "width": 64,
    "height": 64
  }
}
```

> **Important**: `width` and `height` must be divisible by 16.

**Workflow:**
1. **Define**: Add assets to `image_prompts.json`
2. **Develop**: Game uses placeholder if file missing
3. **Generate**: Create images from prompts
4. **Deploy**: Place images in `assets/`, game picks them up

---

## Checklists

### New Game

1. [ ] `cargo new game_name`
2. [ ] Add dependencies to `Cargo.toml`
3. [ ] Create folder structure (`src/state/`, `src/data/`, etc.)
4. [ ] Implement `GameState` and `StateTransition` enums
5. [ ] Create `Game` struct with update/draw loop
6. [ ] Set up `assets/` folder
7. [ ] Copy `publish.ps1` from template
8. [ ] Create `index.html` with correct WASM filename
9. [ ] Implement save/load system

### Migration (Web → Rust)

1. [ ] Define Rust structs for game entities
2. [ ] Set up `macroquad::main` entry point
3. [ ] Copy `publish.ps1` and `index.html` from template
4. [ ] Port PHP/backend logic to Rust functions
5. [ ] Rebuild React UI using immediate-mode
6. [ ] Migrate MySQL data to JSON or SQLite
7. [ ] Wire UI to modify game state

---

## Non-Goals

- ❌ No ECS overengineering
- ❌ No custom editor tooling (initially)
- ❌ No procedural generation until core stable

> **Simplicity is a feature.**
