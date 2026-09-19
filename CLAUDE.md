# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Tradeborn Realms** — a fantasy securities market in Rust + `macroquad`, part of the
`RustGames` catalog. You are a broker on the Tradeborn Exchange; every listing is a claim
on an entity in a **read-only projection of the Mytherra world simulation** (`../mytherra/`).

**Status: M0-M2 complete — the exchange trades.** The template's grid/fog/energy starter is
gone. `src/world.rs` is the seam, `src/world/live.rs` reads the shared Mytherra world over
HTTP, `src/world/fundamentals.rs` reads a `WorldView` into the exchange's one vocabulary, and
`src/world/delta.rs` diffs two such readings into typed changes. `src/market/instruments.rs`
turns a reading into a `Board` of listings with fair values; `src/market/impact.rs` turns
typed deltas into shocks; `src/market/pricing.rs` moves each price by drift + shock + noise +
flow; `src/market/orders.rs` prices an order including its own market impact; and
`src/state/portfolio.rs` is the ledger. A century can be traded headlessly, and `src/ui/`
carries five screens: the floor (board, ledger, movers, settlements, desk with an order
ticket), a listing's own page (chart, fundamentals, price-move decomposition), the chronicle,
the research browser, and the records (equity curve, the licence ladder, honours, the seeds).
`publish.ps1` runs clean end to end and the WebGL build has been verified in a browser.
What remains is a **balance pass** (see *Known gaps*) and audio via `synth`. `gdd.md` is the
specification and the source of truth.

**One deliberate deviation from the GDD.** The GDD makes Solo (an embedded, locally ticked
`mytherra-core`) the default and Live the last milestone (M6). This project went **Live
first, by explicit decision** — the point is to trade the shared world every deity is
nudging. The seam is built exactly as §11 specifies, so a Solo implementation can be added
behind `WorldSource` later without touching anything downstream; it simply does not exist
yet. Anything in the GDD that assumes Solo ships first is stale.

Workspace-wide conventions (800-line file limit, no `mod.rs`, toolkit-first, commit style,
publishing) live in `../rust_management/CLAUDE.md` and the four synced docs in this
directory. Everything below is what's specific to this project.

## Commands

**The game needs a running world.** Start `mytherra-server` first, from `../mytherra`:

```powershell
cd ..\mytherra ; .\run-server.ps1     # needs XAMPP MySQL up; the DB is the save
```

Then, with the spectator secret in the environment (see *Configuration* below):

```powershell
$env:TB_REALMS_SPECTATOR_TOKEN = '<SPECTATOR_TOKEN from mytherra-server/.env>'

cargo run                                                  # native debug, reads 127.0.0.1:8791
cargo test                                                 # all tests (no server needed)
cargo test a_house_losing_its_legend                       # one test by name
cargo test -- --ignored --nocapture a_turning_of_the_world # end-to-end; waits out a server tick
cargo fmt -- --check                                       # CI enforces
cargo clippy --all-targets --all-features -- -D warnings   # CI treats warnings as errors
cargo build --release --target wasm32-unknown-unknown      # WebGL

.\publish.ps1                                              # build Windows + WebGL, deploy (the sanctioned validation path)
.\scripts\capture_ui.ps1 -Scenes boards,trading,listing,chronicle,research,records
```

The ignored live-world tests need a server; other tests run offline, including
delta fixtures built from a seeded `WorldState` projected through spectator
`Standing`. The existing source-owned suites are legacy migration work: move
all tests and test-only helpers into this crate's `tests/` directory, using the
public library API, before expanding coverage. Follow `CODE_STANDARDS.md` §11
for the five-case target across each major feature and justified exceptions.

Never set `RUSTFLAGS` by hand — wasm link flags live in the workspace `.cargo/config.toml`
and an env var replaces rather than merges them, forking the whole wasm dependency cache.

CI (`.github/workflows/rust-ci.yml`) is the synced shared workflow; it checks out
`macroquad-toolkit` as a sibling because a standalone game repo has no workspace.

## The Mytherra tie-in — the defining architectural fact

`../mytherra/` is **its own Cargo workspace**, `exclude`d from the RustGames root workspace.
`tb_realms` takes **cross-workspace path dependencies** on `../mytherra/mytherra-core` (the
entity types) and `../mytherra/mytherra-protocol` (the wire types). Both crates already
depend on the same `macroquad-toolkit` path this game does, so Cargo unifies to one copy.

The single type everything downstream reads is `mytherra_protocol::WorldView` (year,
tick_count, `revealed: BTreeSet<VisibilityScope>`, summary, era, plus `heroes`, `houses`,
`orders`, `regions`, `settlements`, `resource_nodes`, `trade_routes`, `artifacts`, …).

### Two changes this game required in the mytherra repo

The GDD claimed "v1 needs no change to Mytherra at all." That was wrong, and both fixes are
already made and committed there (`A watcher at the window…`, `ac8067e`):

1. **`houses` and `orders` are now on `WorldView`.** They existed on `WorldState` but
   `project()` never put them on the wire, so house scrip and order charters — the two
   highest-alpha classes — were unreachable. Both are revealed with `VisibilityScope::Heroes`.
2. **The server has read-only spectator endpoints.** `POST /session` mints a level-1 deity,
   which `tier_unlock_levels` puts at `Watcher`: heroes only, no regions/settlements/
   resources/routes. A broker cannot climb out of that, because climbing means taking divine
   actions. So `GET /spectate` and `GET /spectate/events?since=` serve
   `spectator_standing()` — every scope, **no verbs and no betting markets**. The empty verb
   set is what makes it safe, not the endpoint's name.

If either is reverted upstream, this game loses most of its instrument taxonomy. The
`#[ignore]`d `live_world_reads_the_shared_world…` test is what catches that.

### Rules that fall out of it

- **`src/world.rs` is the most important file in the project.** One trait, `WorldSource`,
  with `poll`/`view`/`cursor`/`resume_from`/`status`. Everything above it is mode-agnostic;
  `game.rs` holds it as `Box<dyn WorldSource>` and does not know a wire exists.
- **`world/fundamentals.rs` is the one reading of the world**, and both the delta and the
  pricer go through it. `Snapshot::read(view)` yields per-entity `Field`-keyed values;
  `delta::diff_snapshots` compares two of them and `market` values one of them. Reading the
  world twice, separately, is what would make a listing's decomposition stop adding up — the
  ticker telling one story and the fundamentals another. Add a new watched number *there*,
  not in either consumer.
- **The seam is poll-shaped, not blocking, and that is deliberate.** A networked source
  cannot answer "give me the world" synchronously without stalling the frame. An in-process
  source satisfies a poll trivially by always being ready; retrofitting a blocking trait to
  a network is what forks a codebase.
- **Price the numeric deltas, never the prose.** Mytherra's `WorldEvent` is
  `{ year, kind, message }` with four coarse `kind` variants and free-text substance.
  `delta.rs` diffs consecutive `WorldView`s for typed field changes; the chronicle rides
  along in `WorldDelta::events` as the newspaper and **nothing that prices may read it**.
- **Houses and orders are priced on their *living* membership**, which is not a field either
  carries — it is reconstructed against the hero roster. `member_ids` is the whole lineage
  including the dead, so its length only grows; a house that loses one of two members can
  lose nine tenths of its renown, and `LivingMembers` vs `LivingRenown` is what tells those
  apart.
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

### Screens (GDD 9)

`ui.rs` owns the vocabulary (`UiAction`), the context, and the layout constants; each panel is
its own file under `ui/`. `GameSession::screen()` picks the screen; the nav sits in the header, and
`Esc` or the Back button leaves a listing's page.

**The chronicle screen is Pillar 4 as a layout.** The world's prose runs down the left; what
the floor actually did that year runs down the right; *nothing joins them but the year*,
because nothing that prices may read a word of the chronicle. The cross-link of GDD §9 runs
from a **price move** to the paper that moved — never from a line of prose to a listing, since
nothing in the game knows what the prose is about. Each ticker opens that listing's page.

The chronicle is seeded from the first `WorldView`'s own archive (`seed_chronicle`, once only)
so the exchange opens with the recent newspaper already on the desk. The world has a past
whether or not this session watched it happen.

**The chart draws the envelope, not a line.** Each `series` bucket carries the high and low it
reached, so the plot is a filled band with the closing value traced over it — a line through
closes alone would step neatly between decimated buckets and imply a calm that never happened.
It also draws **price against fair value on one pair of axes**, because the gap between them is
what a broker is actually trading; the scale spans both series, or fair value would leave the
plot exactly when it diverged, which is when the chart matters most. The approach is taken from
`dragons_hoard/src/ui/history.rs`, which solved it first — don't re-derive it.

### Licences and ages (GDD 5.3, 5.6)

`assets/data/licences.json` is the information ladder — the game's real progression axis, since
**information is what you spend on, not capital** (Pillar 3). Four cumulative rungs; the first
is free; every listing class must appear under exactly one rung or paper would exist no licence
could reach. Buying a rung spends the same cash that buys paper, which is the whole tension.

**An honest note on the gate.** GDD §5.3 maps the tiers onto Mytherra's `VisibilityScope`, and
they are named against it — but the spectator projection already carries *every* scope (that is
what makes the endpoint safe), so a licence cannot make the exchange unsee what the server sent.
The gate is the exchange's own: it decides what a broker of this rank is shown and may trade.
A Solo world source could enforce the scopes for real, and the tiers are shaped so it would slot
in without redesign. Don't let the doc comments imply a server-side filter that isn't there.

**An age ending is detected, never predicted.** `EraState.number` changing between readings is
the only signal; `Observed.cataclysm` reports it, and the session guards against noticing the
same one twice. `Instrument::era_history` is a **third `Series`**, decimated in step with price
and fair value, so the chart can rule off where an age ended: a bucket whose min and max differ
is a bucket an era turned inside. A list of boundary years could not have survived decimation.

### Honours (GDD 5, M7)

`state/honours.rs` decides every achievement in **one place**, from the session's own facts,
run once per settlement and once per fill. Scattering unlocks across call sites is how the same
honour gets awarded twice by two paths that each thought they owned it.

Nothing pays out. The exchange notices and that is all — a reward would make honours a second
progression axis competing with the licences, which are meant to be the only one (Pillar 3).

### Keyboard

Everything the mouse can do, a keyboard can: `1-4` for the screens, arrows to walk the board,
`Enter` to open a listing, `B`/`X` to buy and sell, `[`/`]` to resize the ticket, `Esc` back.
**There is deliberately no key to advance the world** — it turns on its own clock and nothing
this game accepts can hurry it. `game_page.json` used to promise a "Space: advance the
settlement" control that could never have existed; if a control is listed there, make sure it
is real.

## Configuration

`assets/data/game_config.json` is embedded at compile time via `include_str!` in
`src/data.rs`. It carries the world source, not yet any market balance:

- `server_url` — what a **debug native** build reads (`http://127.0.0.1:8791`).
- `gateway_url` — what a **published** build reads instead
  (`https://webhatchery.au/local_gateway/api/p/mytherra`). Mirrors mytherra's own field and
  exists for the same reason: a browser on an https page cannot call a plain-http server, so
  the WebGL build has no other route. `GameConfig::world_url()` picks between them.
- `spectator_token` — **committed blank on purpose.** On native it is overridden by
  `TB_REALMS_SPECTATOR_TOKEN`, so no secret is in git. A WebGL build has nowhere secret to
  keep one and needs it filled in before publishing — at which point anyone can read it out
  of the `.wasm`, which is exactly why it may only ever guard a *read-only* projection.
  A blank token is a loud configuration error, never a silent 401.
- `TB_REALMS_SERVER_URL` overrides the URL on native.

`assets/data/instruments.json` is the taxonomy and the fair-value rule per class (GDD §5.2,
§5.4), also embedded via `include_str!`. Its shape:

```
fair_value = base_value + Σ terms + Σ scarcity , floored at floor_value,
             then × every modifier whose condition the entity currently meets
```

- `terms` — `weight × fundamental`, the ordinary case.
- `scarcity` — `weight ÷ max(fundamental, 1)`. **This is how a commodity prices a supply
  shock**: a storm that ruins the fields does not make grain worth less. Commodities are the
  only class that uses it, and it is why they are aggregated per `ResourceType` rather than
  per node.
- `modifiers` — a multiplier while a flag or status reads a given way, keyed on the world's
  own variant name (`WarTorn`, not `war_torn` — the labels come from `Debug`).
- `income` — real per-settlement income, so `Instrument::yield_rate()` is measured rather
  than declared. Only bonds, shares and tolls pay one.

`InstrumentRules::validate()` runs at load and refuses to open the exchange if a class is
missing, duplicated, or prices on a key `Field::from_key` does not know — the GDD §14
no-orphan-instrument rule enforced at load rather than discovered in play.

`assets/data/impacts.json` is the state-delta -> price-shock table (GDD §5.5) and the balance
surface of the whole game. A rule keys on a `Field`, an instrument class, and a **direction of
travel** along the world's own parent link (`Fundamentals::parent`):

- `self` — the listing's own entity moved.
- `parent` — the listing is the moved entity's parent. A resource node is not a listing, so
  this is the *only* route its status change has to the board, and it reaches the commodity
  above it.
- `child` — the listing is a child of the moved entity. A famine is a fact about a province
  and the towns inside it are what starve.

Levels belong in `instruments.json` and reach price as drift; `impacts.json` is only the
jolt. A mine working out therefore appears in both, on purpose: the jolt is immediate, the
revaluation is permanent.

The `orders` block of `game_config.json` is the execution model. **Size costs twice** — as
slippage on the fill, and as a `flow` mark left on the price that decays over following
settlements. `OrderConfig::depth(liquidity)` is where GDD §5.2's promise that relic lots have
"enormous spreads" is actually kept. `orders::quote` is used for both the ticket's estimate
and the execution, deliberately: two code paths there would be a way to lie to the player
about their own costs.

Still planned per GDD §6: `strings.json`.

**`impacts.json` is the balance surface of the entire game and must never become Rust
constants** — it is where a designer tunes how hard a famine hits grain. `Field::key()` is
the vocabulary it will key on, every key is asserted distinct, and every key round-trips
through `Field::from_key`.

`asset_packs.json` zips `assets/` into `assets.zip` at publish time (`delete_source: true`).

## Screenshot capture

`src/main.rs` wires `macroquad_toolkit::capture` under the **`TB_REALMS`** env prefix. Setting
`TB_REALMS_CAPTURE_PATH` boots a named scene, simulates fixed frames at a fixed timestep,
writes a PNG, and exits (compiled out on wasm32). `Game::begin_capture_scene` holds the scene
until the board is non-empty, because every screen here is downstream of the world answering.
Scenes: `boards` (anything unrecognised) shows the opening state; **`trading` selects the
deepest listing and buys it**, so one screenshot covers the ticket, the fill, the tape and a
real position in the ledger — the whole order path, which no static scene shows; `listing`
does the same and then opens that listing's page; `chronicle`, `research` and `records` open those.

A capture runs in about a second of wall-clock, while the server ticks every ten, so a
captured frame will normally show a full board but **zero settlements observed**. That is
correct, not a bug: the board hangs from the first view to arrive, while a settlement is
only logged when the world actually turns, and the world's clock is not the exchange's to
hurry.

## CI needs two repos

This game builds from a standalone checkout in CI, so the path dependencies on `../mytherra`
cannot resolve unless that repo is fetched alongside it. That extra checkout is **declared in
`rust_management/sync-rust-ci-workflows.ps1`'s `$ExtraCheckouts` map**, not hand-edited into
`.github/workflows/rust-ci.yml` — the sync script overwrites project-local copies, so a
hand-edit would survive exactly until the next sync. Change it there and re-sync.

## Known gaps

- No prices, orders, portfolio or charts (M2–M3). The board shows fair value, which is not a
  price and never will be.
- **`instruments.json` is untuned against an old world.** The weights were set against a
  freshly seeded Mytherra; the shared live world is past year 20,000, where settlements have
  grown to carrying capacity and most resource nodes are worked out. Listings there span
  ~40 to ~1050 with settlement shares dominating the top of the board. That is a balance
  pass for M2, not a fault in the derivation.
- `LiveWorld` re-reads the entire `WorldView` every poll — around 900 heroes' worth of JSON
  at present. Fine at a five-second cadence; if it stops being fine, the fix is a
  delta endpoint on the server, not a cache here.
- `AGENTS.md`, `CODE_STANDARDS.md`, `MACROQUAD_TOOLKIT.md`, and `GAME_DEVELOPMENT_GUIDE.md`
  are synced copies of `rust_management/docs/` — never hand-edit them. Project-specific
  guidance goes in this file, `README.md`, or `gdd.md`.
- Don't add an `index.html`; the page is generated at publish time from `game_page.json` plus
  the shared `rust_management/web/` shell.

## Milestone order

GDD §13 is M0 seam → M1 instruments → M2 market → M3 floor UI → M4 chronicle/research →
M5 licences and eras → M6 Live mode → M7 polish.

**M0, M1 and M2 are done, and M6 was folded into M0** — Live is the mode that exists. What
remains in GDD order: **M3 the floor** — a listing screen with a chart drawn from
`Instrument::history` via `macroquad_toolkit::series`, then M4's chronicle and research
browser.

`market/` must not depend on macroquad, and `state/portfolio.rs` must not either — the
century-long backtest of GDD §14 runs through both and is the test that catches balance
regressions no screenshot can. Keep it that way.

The GDD's "the game must be complete and shippable without a server ever running" no longer
holds as written: without a server there is no world. Restoring it means writing the Solo
`WorldSource`, which the seam is built for but which nobody has written.
