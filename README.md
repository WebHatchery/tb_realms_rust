# Tradeborn Realms

A fantasy securities market. You are a broker on the floor of the Tradeborn
Exchange, holding positions in noble houses, knightly orders, regions, trade
routes, and commodities — every one of them backed by a real entity in a living
world you cannot touch.

**Status:** the world seam is live; the market is not built. The exchange
connects to a running Mytherra world, reads it, and reports each turning as
typed changes — but there are no instruments, prices, or positions yet. Read
[`gdd.md`](gdd.md); it is the source of truth for what remains.

Ported from `game_apps/tb_realms/` (React 19 + PHP/Slim + MySQL). See §0 of the
GDD for the mechanic carry-over table and what was cut.

## The Mytherra tie-in

Tradeborn Realms does not simulate a market in a vacuum — it prices a **read-only
projection of the Mytherra world simulation** (`../mytherra/`). Houses and Orders
carry a `prestige` scalar that drifts toward the summed renown of their living
members; regions carry `prosperity`, `harvest`, `population`, `chaos`; trade
routes carry `volume`; resource nodes have a type and a status. Those are the
fundamentals the exchange prices.

The GDD describes two modes. **Only Live is built**, by decision — the point is
to trade the shared world every deity is actually nudging:

| Mode | World source | State |
| --- | --- | --- |
| **Live** | `mytherra-server`'s `GET /spectate` + `GET /spectate/events?since=` | Built. Trades the shared world; needs a running server. |
| **Solo** | An embedded `mytherra-core` world ticked locally | Not written. The seam is built for it — `world.rs` is one trait with room for a second implementation. |

Live reads through the server's **read-only spectator endpoints**, which grant
every visibility scope and no action verb whatsoever. Tradeborn Realms sees the
whole world and can change none of it — see GDD §12 for why that is a design
pillar rather than a limitation.

## Build and run

The exchange needs a world to read, so start the authority server first:

```powershell
cd ..\mytherra ; .\run-server.ps1     # needs XAMPP MySQL; the DB is the save
```

Then, with the spectator secret from `mytherra-server/.env`:

```powershell
$env:TB_REALMS_SPECTATOR_TOKEN = '<SPECTATOR_TOKEN>'

cargo run                     # native debug build
cargo test                    # tests (no server needed)
cargo test -- --ignored       # end-to-end against the running server
cargo fmt -- --check          # CI enforces this
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --target wasm32-unknown-unknown   # WebGL
```

A published build reads `gateway_url` instead, so the browser reaches a
home-run server through one https origin. That build needs `spectator_token`
filled into `assets/data/game_config.json` — it is committed blank so no secret
lives in git, and it may only ever guard a read-only projection because anyone
can read it out of the deployed `.wasm`.

## Publish

```powershell
.\publish.ps1                 # build Windows + WebGL, deploy to the local preview root
.\publish.ps1 -WebGLOnly      # -WindowsOnly, -DeployOnly, -Production (-p), -FTP, -DryRun
```

## Screenshots

```powershell
.\scripts\capture_ui.ps1 -Scenes boards
```

Drives the headless capture harness through the `TB_REALMS_CAPTURE_*` env vars
wired in `src/main.rs`. Output lands in `docs/verification/`. A capture takes
about a second while the server ticks every ten, so it will normally show no
settlements observed — the world's clock is not the exchange's to hurry.

## Project docs

`AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and
`GAME_DEVELOPMENT_GUIDE.md` are synced copies of the canonical versions in
`rust_management/docs/` — don't hand-edit them. Project-specific guidance goes
here or in `gdd.md`.
