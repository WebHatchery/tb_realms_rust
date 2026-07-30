# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Tradeborn Realms** — a fantasy securities market in Rust + `macroquad`, part of the
`RustGames` catalog. You are a broker on the Tradeborn Exchange; every listing is a claim
on an entity in a **read-only projection of the Mytherra world simulation** (`../mytherra/`).

**Status: scaffold.** Everything in `src/` is still the unmodified
`rust_management/template/` starter — tile grid, fog of war, an energy/points action loop.
**None of it is the game.** `gdd.md` is the specification and the source of truth for what
to build; read it before touching `src/`. The template's grid/fog scaffolding is explicitly
slated for removal (GDD §10), so don't extend it — replace it.

Workspace-wide conventions (800-line file limit, no `mod.rs`, toolkit-first, commit style,
publishing) live in `../rust_management/CLAUDE.md` and the four synced docs in this
directory. Everything below is what's specific to this project.

## Commands

```powershell
cargo run                                                  # native debug, solo mode
cargo test                                                 # all tests
cargo test legacy_save_migrates_to_current_shape           # one test by name
cargo fmt -- --check                                       # CI enforces
cargo clippy --all-targets --all-features -- -D warnings   # CI treats warnings as errors
cargo build --release --target wasm32-unknown-unknown      # WebGL

.\publish.ps1                                              # build Windows + WebGL, deploy (the sanctioned validation path)
.\scripts\capture_ui.ps1 -Scenes floor,listing,portfolio   # headless screenshots -> docs/verification/
```

Never set `RUSTFLAGS` by hand — wasm link flags live in the workspace `.cargo/config.toml`
and an env var replaces rather than merges them, forking the whole wasm dependency cache.

CI (`.github/workflows/rust-ci.yml`) is the synced shared workflow; it checks out
`macroquad-toolkit` as a sibling because a standalone game repo has no workspace.

## The Mytherra tie-in — the defining architectural fact

`../mytherra/` is **its own Cargo workspace**, `exclude`d from the RustGames root workspace.
`tb_realms` takes **cross-workspace path dependencies** on `../mytherra/mytherra-core` (the
headless sim) and `../mytherra/mytherra-protocol` (the wire types). This was verified to
resolve and build cleanly (GDD §5.1, 2026-07-26); the deps were then *reverted* because
nothing uses them yet and the standards forbid unused dependencies. **Re-add them in M0 with
the first line of code that needs them.**

Both crates already depend on the same `macroquad-toolkit` path this game does, so Cargo
unifies to one copy — no feature-unification problem.

The single type everything downstream reads is `mytherra_protocol::WorldView` (year,
tick_count, `revealed: BTreeSet<VisibilityScope>`, summary, era, plus `heroes`, `regions`,
`settlements`, `resource_nodes`, `trade_routes`, `weather`, `wars`, `artifacts`, …).

### Rules that fall out of it

- **`src/world.rs` is the most important file in the project.** One trait — *give me the
  current `WorldView` and the events since cursor N* — with a Solo impl (embedded
  `mytherra-core`, ticked in-process) and a Live impl (`quad-net` against `mytherra-server`).
  Everything above it must be mode-agnostic. Get the seam wrong and the two modes fork the
  codebase.
- **Solo is the default and the balanced mode**, not a fallback. It is what ships in WebGL
  and what CI tests. Live mode is milestone M6, deliberately last.
- **Price the numeric deltas, never the prose.** Mytherra's `WorldEvent` is
  `{ year, kind, message }` with four coarse `kind` variants and free-text substance.
  The price oracle diffs consecutive `WorldView`s for typed field changes (harvest fell,
  a house lost its highest-renown member, a route's volume halved). The chronicle renders
  *alongside* as flavour. Parsing chronicle strings for signal is the mistake this design
  exists to prevent.
- **Strictly one-directional: this game never mutates Mytherra.** No `PlayerAction`, no
  nudges, no write path of any kind. It is a design pillar, not an omission.
- Live mode HTTP must follow `mytherra/src/net.rs`'s poll-based `ServerClient` exactly —
  do not invent a second cross-platform HTTP pattern.

## Architecture the GDD commits to

Target layout (GDD §11) — `game.rs` state machine, `world.rs` + `world/{solo,live,delta}.rs`,
`market.rs` + `market/{instruments,pricing,impact,orders}.rs`, `data.rs`, `state.rs`, `ui.rs`
+ `ui/`, and `exchange_actions.rs` as the `UiAction` dispatcher.

- **`market/` must not depend on `macroquad`.** The headline test is a headless backtest
  running a full century with a scripted strategy and asserting the equity curve stays in
  bounds; that only works if the market layer is engine-free.
- **UI is a pure view layer** returning `UiAction`; the dispatcher applies them. Never mutate
  state from a panel — add a `UiAction` variant.
- **Two separate seeded `macroquad_toolkit::rng` streams** (world sim, market noise/order
  flow), kept apart so they cannot desynchronise each other. Both seeds are saved and shown
  on the records screen; same seeds + same orders → identical price history. This is what
  makes the determinism and backtest tests possible.
- **`macroquad_toolkit::series` is the headline toolkit use** — a bounded time series that
  never loses an extreme, built for exactly this. Expect to push chart improvements (axis/tick
  labelling, sortable-table widget) *back into the toolkit* rather than solving them locally.
- No textures, no camera, no sprites, no map. This game is deliberately UI-only — "you are
  looking at the world through paper." The chart is the art.

## Data

`assets/data/*.json` is embedded at compile time via `include_str!` in `src/data.rs` and
loaded through `load_embedded_json_labeled` / `DataRegistry`. `asset_packs.json` zips
`assets/` into `assets.zip` at publish time (`delete_source: true`), loaded at runtime by
`AssetManager::load_asset_pack`.

Planned data files per GDD §6: `instruments.json` (fair-value rules per class),
`impacts.json`, `licences.json`, `achievements.json`, `strings.json`, `world_seed.json`.

**`impacts.json` is the balance surface of the entire game and must never become Rust
constants** — it is where a designer tunes how hard a famine hits grain. The same applies to
the fair-value and volatility rules in `instruments.json`.

The current `game_config.json` fields (`starting_points`, `energy_per_second`,
`world_width`/`height`) are template leftovers; the real schedule is starting capital, fees,
settlement length, margin rules, and licence costs.

## Screenshot capture

`src/main.rs` wires `macroquad_toolkit::capture` under the **`TB_REALMS`** env prefix. Setting
`TB_REALMS_CAPTURE_PATH` boots a named scene, simulates fixed frames at a fixed timestep,
writes a PNG, and exits (compiled out on wasm32). Right now there is one boot state, so
`-Scenes` only picks the output filename — wiring real scenes (`floor`, `listing`,
`portfolio`, …) via `begin_capture_scene` is part of building the screens.

## Gaps in the scaffold

- Not a git repo yet (`git init` before the first commit; commit style is in
  `rust_management/docs/COMMIT_STYLE.md`, with `mytherra` as the worked exemplar).
- No root `catalog_thumbnail.png` — CI warns, and `publish.ps1` deploys it for the catalog card.
- `AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and `GAME_DEVELOPMENT_GUIDE.md`
  are synced copies of `rust_management/docs/` — never hand-edit them. Project-specific
  guidance goes in this file, `README.md`, or `gdd.md`.
- Don't add an `index.html`; the page is generated at publish time from `game_page.json` plus
  the shared `rust_management/web/` shell.

## Milestone order (GDD §13)

M0 the world seam → M1 instruments → M2 the market → M3 the floor UI → M4 chronicle/research
→ M5 licences and eras → M6 Live mode → M7 polish. The game must be complete and shippable
without a server ever running.
