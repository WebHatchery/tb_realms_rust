# Tradeborn Realms

A fantasy securities market. You are a broker on the floor of the Tradeborn
Exchange, holding positions in noble houses, knightly orders, regions, trade
routes, and commodities — every one of them backed by a real entity in a living
world you cannot touch.

**Status:** scaffold. This is the `rust_management/template/` starter with the
project's identity applied — no game code has been written yet. Read
[`gdd.md`](gdd.md) first; it is the source of truth for what to build.

Ported from `game_apps/tb_realms/` (React 19 + PHP/Slim + MySQL). See §0 of the
GDD for the mechanic carry-over table and what was cut.

## The Mytherra tie-in

Tradeborn Realms does not simulate a market in a vacuum — it prices a **read-only
projection of the Mytherra world simulation** (`../mytherra/`). Houses and Orders
carry a `prestige` scalar that drifts toward the summed renown of their living
members; regions carry `prosperity`, `harvest`, `population`, `chaos`; trade
routes carry `volume`; resource nodes have a type and a status. Those are the
fundamentals the exchange prices.

Two modes, and the offline one is not a fallback afterthought — it is the
default:

| Mode | World source | Notes |
| --- | --- | --- |
| **Solo** (default) | An embedded `mytherra-core` world ticked locally | No server, no account, fully deterministic from a seed. Ships in the WebGL build. |
| **Live** (opt-in) | `mytherra-server`'s `GET /view` + `GET /events?since=` | Trades the same shared world every deity is nudging. Requires a running server. |

Both read the **same** `mytherra-protocol::WorldView` type, so there is one price
oracle, not two. See GDD §5.1 and §11 for the crate wiring, and §12 for what
this deliberately does *not* do (Tradeborn Realms never mutates Mytherra).

## Build and run

```powershell
cargo run                     # native debug build, solo mode
cargo test                    # tests
cargo fmt -- --check          # CI enforces this
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --target wasm32-unknown-unknown   # WebGL
```

## Publish

```powershell
.\publish.ps1                 # build Windows + WebGL, deploy to the local preview root
.\publish.ps1 -WebGLOnly      # -WindowsOnly, -DeployOnly, -Production (-p), -FTP, -DryRun
```

## Screenshots

```powershell
.\scripts\capture_ui.ps1 -Scenes floor,listing,portfolio
```

Drives the headless capture harness through the `TB_REALMS_CAPTURE_*` env vars
wired in `src/main.rs`. Output lands in `docs/verification/`.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.
