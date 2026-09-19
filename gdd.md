# Tradeborn Realms — Game Design Document

*Draft v0.1 — living document.*

> Every listing on the Tradeborn Exchange is a claim on something that actually exists: a
> house whose worth is the renown of heirs who can die, a region whose granaries can
> empty, a mine a war can close. You cannot nudge the world — that is what the gods are
> for, and they are not you. You can only read it faster than the broker beside you, and
> be holding the right paper when the age ends.

Sources: `game_apps/tb_realms/` (React 19 + PHP/Slim + MySQL original),
`RustGames/mytherra/` (the simulation this game prices),
`rust_management/standing.md`,
`rust_management/docs/GAME_DEVELOPMENT_GUIDE.md`, `rust_management/docs/CODE_STANDARDS.md`,
`rust_management/docs/MACROQUAD_TOOLKIT.md`.

---

## 0. Migration Snapshot

- **Old game:** `game_apps/tb_realms/` — a React 19 SPA (~10.7k LOC) over a PHP/Slim +
  MySQL backend with a real schema (stocks, portfolios, transactions, watchlists,
  market_events, achievements) behind the shared WebHatchery login.

- **Why it was picked:** Fantasy market speculation provides a different loop
  from `auction_game`'s individual auctions. Charts and numbers carry the
  decisions with modest artwork requirements. The toolkit's existing
  `series.rs` provides bounded time-series storage that preserves extrema.

- **The honest finding that shapes this whole port.** Two things are true about the
  original and both matter more than any feature list:

  1. **There is no market simulation.** `StockActions::updateStockPrices` persists price
     updates that nothing ever generates. No random walk, no volatility model, no event
     → price pipeline. Prices are the five values in the seed migration and they never
     move. The game is a portfolio UI over a static table.
  2. **The fantasy is a skin over an equities app.** The entity model is
     `peRatio`/`dividendYield`/`beta`/`marketCap`, the sectors are `Technology`,
     `Healthcare`, `Real Estate`, `Consumer Defensive`, and the event types are
     `earnings_report` and `merger_acquisition`. The seed listings are "TechCorp
     Industries" and "Divine Healing Arts, Healers Guild". The README promises trading in
     "guilds, magical commodities, mythical beasts"; the schema delivers NASDAQ with a
     `guild` column bolted on.

  So this port is not a translation. **The old game supplies a competent trading UI and a
  data model; it supplies no world and no price formation.** Both of those are the actual
  game, and `mytherra` already has them. That is the entire argument for the tie-in in §1
  and §5.

- **Art-liability audit.** The project ships **zero image files** outside `node_modules`.
  No canvas, no charting library with sprite assets, no icons beyond CSS.

  | Old asset (web) | Art cost | Rust replacement |
  | --- | --- | --- |
  | Price charts (CSS/DOM) | None | `macroquad_toolkit::series` + `ui` primitives. **The chart is the art** |
  | Listings table, portfolio, transaction ledger | None (CSS) | `ui` `GridLayout`, scroll lists, `TextStyle` |
  | Achievement icons (`icon: string` — an emoji or CSS class) | None | Text badges via `colors` |
  | `MarketEvent.imageUrl` | None — declared, never populated | Cut the field |
  | Sector/guild colour coding | None (CSS) | `colors` palette |

  **Nothing requires an artist.** This is the cheapest game to finish in the entire
  migration shortlist, and the finish cost is typography, layout, and balance.

- **Mechanic carry-over table.**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Instrument model (symbol, price, previous close, change, volume, market cap, 52-week high/low, price history) | Keep, re-found | Shape is right; the *meaning* changes from invented equities to claims on Mytherra entities (§5.2) |
  | `peRatio` / `dividendYield` / `beta` | **Redesign** | Ported as fantasy-native analogues with real definitions: yield is actual per-tick income, beta is measured covariance with the world index, and the P/E analogue becomes prestige-to-price. Never decorative numbers |
  | Real-world sectors (Technology, Healthcare, Real Estate…) | **Cut** | Replaced by the instrument taxonomy in §5.2, which is drawn from the world model rather than invented |
  | Buy/sell, portfolio, positions, cost basis, P&L | Keep as-is | Solid and complete |
  | Watchlist | Keep | |
  | Transaction ledger | Keep | |
  | Market events (8 types, 4 severities, affected-stock impact vectors) | **Redesign** | Ported as *derived* events — read from the world's state deltas rather than authored. See §5.4 |
  | News page | Keep, re-source | Becomes Mytherra's chronicle, which is authored prose the sim already generates |
  | Achievements (categories, difficulty, points, progress) | Keep | Via toolkit `achievements` |
  | Leaderboard | **Cut for v1** | Requires a server and other players; the offline analogue is a personal records screen. Revisit if Live mode (§5.1) grows a shared exchange |
  | Profile / Settings / Auth / guest sessions | Cut | Local slots via `persistence`. Account linking only matters in Live mode, and Mytherra already owns that flow |
  | `useMockData` / `NOT_IMPLEMENTED` service branches (achievements, market events — dozens of sites) | Cut | Scaffolding for an implementation that never landed |
  | `TimeSimulationParams { days, speed: 'normal' \| 'fast' \| 'ultra' }` | Keep the intent | The one place the original acknowledged time must pass. Becomes the settlement clock in §5.6 |
  | PHP/MySQL backend, shared login | Cut | Replaced by an embedded world in Solo mode, or Mytherra's server in Live mode |

- **Explicitly out of scope:** real money, anything resembling a real security or a real
  exchange, gambling mechanics with purchasable credits, PvP trading, and — stated
  precisely because it is the tempting mistake — **any ability for this game to change
  Mytherra**. See §12.

---

## 1. High Concept

- **Pitch:** You are a broker on the floor of the Tradeborn Exchange. Every listing is a
  claim on a living world you have no power over — a bloodline, a knightly order, a
  province, a trade road, a mine. Read the chronicle, price the fundamentals, take your
  position, and survive the century.
- **Genre:** Trading / market simulation. No analogue in `standing.md`. Adjacent to
  `auction_game` (one-shot bidding) and to Mytherra's own Observatory (binary wagers) but
  mechanically distinct from both — see §5.7.
- **Perspective & presentation:** UI-only. Charts, ledgers, order tickets, a chronicle
  feed. No world view, no camera, no sprites — **you are deliberately looking at the
  world through paper.**
- **Tone:** Mercantile and slightly ghoulish. The exchange discusses a famine as a
  position. The chronicle's prose is grand and mythic; the ticker beside it is not, and
  the gap between them is the game's whole sense of humour.
- **Comparables:** *Wall Street Kid* / *Taipan!* (period market loops), *Football Manager*
  (deep systems behind a pure-data UI), *Dwarf Fortress* legends mode (the pleasure of
  reading a simulated world) — except here reading it well pays.
- **Audience:** Systems and numbers players; the `stellar_legacy` audience.
- **Scope:** Full game. See §13.
- **Platforms:** WebGL + native Windows.

---

## 2. Design Pillars

1. **Every price is backed by something real.** No instrument may be a free-floating
   number. Each one traces to a named entity in the world with fundamentals a player can
   inspect. If a price moves, there is a reason, and the reason is findable.
2. **The player is a spectator with a wallet.** Tradeborn Realms never touches the world.
   Powerlessness is the fantasy — you are the one person in the setting who cannot change
   anything and profits anyway.
3. **Information is the progression axis.** Not capital. A richer broker with poor
   information should lose to a poorer one with better. Access to deeper world data is
   what you spend on, and Mytherra's Standing model already implements exactly this.
4. **The chronicle is the newspaper, not the oracle.** Prose tells you what happened;
   the fundamentals tell you what it is worth. A player who reads only headlines should be
   consistently, instructively wrong.
5. **The world does not care.** No rubber-banding, no difficulty scaling, no event that
   fires because the player needs one. The sim runs identically whether the exchange is
   open or not.

---

## 3. Core Loop

**The settlement** is the unit of play — one Mytherra tick (one world year).

1. **Read** — the chronicle since last settlement, plus the movers list.
2. **Research** — open a listing: fundamentals, holdings history, related entities, and
   whatever your Standing lets you see of the underlying region, house, or order.
3. **Position** — place buy/sell orders. Large orders move the price against you (§5.5).
4. **Settle** — advance the world one year. Fundamentals update, prices re-form, income
   pays, margin is checked.
5. **Review** — P&L, the position that hurt, the chronicle line that explained it.

Longer arcs: **eras**. Mytherra's world runs in hundred-year eras that each end in a
cataclysm. The exchange survives across them, and an era transition is the largest,
least-priceable event in the game (§5.6).

---

## 4. Player Role & Verbs

**Verbs:** *read* the chronicle; *research* a listing; *buy* / *sell*; *watch* (add to
watchlist); *borrow* (leverage, once unlocked); *subscribe* (spend on an information tier);
*settle* (advance the year).

**Non-verbs — and this list is a design commitment, not an omission:** the player cannot
nudge a region, bless a hero, found a settlement, start a war, or influence any world
entity in any way. There is no PlayerAction. The exchange is strictly downstream of the
world.

---

## 5. Systems & Mechanics

### 5.1 The World Source — the Mytherra tie-in

Tradeborn Realms consumes a **read-only projection of a Mytherra world**. It ships two
modes over the *same* data type, so there is exactly one price oracle:

| Mode | World source | Availability |
| --- | --- | --- |
| **Solo** (default) | An embedded `mytherra-core` world, seeded and ticked in-process | Always. No server, no account, deterministic from a seed. This is what ships in the WebGL build and what CI tests |
| **Live** (opt-in) | `mytherra-server` via `GET /view` and `GET /events?since=` | When a server is configured. Trades a shared world that real deities are actively nudging |

Both produce a `mytherra_protocol::WorldView`. Everything downstream — fundamentals,
price formation, the chronicle feed, the UI — reads that one struct and cannot tell the
modes apart. Solo is not a degraded fallback; it is the primary mode and the one the game
is balanced against.

**Wiring.** `mytherra-core` is deliberately headless ("No macroquad rendering — only the
pure toolkit utilities"), so `tb_realms` takes path dependencies on
`../mytherra/mytherra-core` and `../mytherra/mytherra-protocol`.

**~~Verify the cross-workspace dependency works before building on it.~~ — Resolved
2026-07-26: it does.** `mytherra/` is its own Cargo workspace and is `exclude`d from the
RustGames root workspace, so this is a cross-workspace path dependency. Adding both crates
to this scaffold and running `cargo check` resolves and builds them cleanly
(`Adding mytherra-core v0.1.0`, `Adding mytherra-protocol v0.1.0`, `Finished dev profile`)
with no workspace conflict and no feature-unification problem — `mytherra-core` already
depends on the same `macroquad-toolkit` path this game does, so Cargo unifies to one copy.
The dependency was then reverted, because nothing uses it yet and the repo's standards
forbid carrying unused dependencies; re-add it in M0 with the first line of code that
needs it.

The fallback, should the live-core route become awkward under WASM (build size, tick cost),
remains: depend on `mytherra-protocol` alone and have Solo mode replay a **recorded world**
— a serialised sequence of `WorldView` snapshots generated offline by mytherra and
committed as an asset. Note this fallback is *also* what makes the §14 mode-parity test
possible, so the recording tooling is worth building regardless of which route Solo takes.

**What this buys, concretely.** The original's missing half was price formation driven by
a world. Mytherra supplies:

- entities with *continuously varying scalars that already behave like valuations* (§5.2),
- a chronicle of authored prose describing why they moved,
- weather, famine, plague, war, and cataclysm as genuine exogenous shocks,
- a hundred-year era structure that gives the campaign natural acts,
- and a Standing/visibility model that is a ready-made information-tier progression (§5.3).

**What Mytherra gets back:** nothing that changes it. This is a strictly one-directional
dependency, by design (Pillar 2). The only thing it may eventually want is a *spectator*
session — a client that reads the world without owning a deity. `POST /session` already
mints a guest, so v1 needs no change to Mytherra at all. Flag as a coordination item, not
a blocker.

### 5.2 The Instrument Taxonomy

Every listing derives from a real entity in `WorldView`. This replaces the original's
invented real-world sectors entirely.

| Instrument class | Backed by | Fundamental (from the sim) | Behaves like |
| --- | --- | --- | --- |
| **House scrip** | `House` | `prestige` — which *already* drifts toward the summed renown of living members | Equity in a bloodline. Spikes when an heir becomes a legend; collapses when a line dies out |
| **Order charters** | `Order` | `prestige` — tracks the size of the living fellowship of a `HeroRole` | A sector index. Buying "the Arcane Circle" is a bet on mages flourishing world-wide |
| **Regional bonds** | `Region` | `prosperity`, `harvest`, `population`, `chaos`, `danger`, `strife` | Sovereign debt. Pays a coupon from prosperity; defaults in famine or conquest |
| **Settlement shares** | `Settlement` | `population`, `prosperity` | Small-cap equity. Thin, volatile, occasionally destroyed outright |
| **Route tolls** | `TradeRoute` | `volume` | Infrastructure income. Dies when either endpoint region collapses |
| **Commodities** | `ResourceNode` aggregated by `ResourceType` | Node count and status per type: Farmland, Mine, Forest, Fishery, Quarry, **Manaspring** | Spot commodities. `WeatherEvent.withers` names the resource types a storm ruins — a direct, already-implemented supply shock |
| **Relic lots** | `Artifact` | Scarcity and the renown of its bearer | Illiquid collectibles. Rarely traded, enormous spreads |

Manaspring is the standout: a magical commodity with no real-world analogue, whose supply
responds to the world's magic systems. It is the listing that proves this is a fantasy
market and not a reskin.

### 5.3 Information Tiers

Mytherra's `Standing` gates a `WorldView` by `VisibilityScope`; `WorldView.revealed`
records which scopes are unlocked so a client can distinguish "locked" from "empty". The
exchange maps its broker licence tiers onto exactly that:

| Licence | Reveals | Trading consequence |
| --- | --- | --- |
| **Floor runner** | `summary`, `era` only | Trade the world index and nothing else. You know the tide, never the boat |
| **Chartered broker** | `regions`, `settlements` | Regional bonds and settlement shares become researchable |
| **Guild factor** | `heroes`, `houses`, `orders` | House scrip and order charters — the highest-alpha instruments |
| **Archivist** | `FullChronicle`, `era_history` | Backtesting. See how a listing behaved through previous cataclysms |

This is Pillar 3 made mechanical, and it costs no new Mytherra work — the projection
already filters this way.

### 5.4 Price Formation

**Price is not the fundamental.** If it were, the game would be arithmetic. Each
settlement, for each instrument:

```
fair_value   = f(fundamentals)              // deterministic from WorldView
drift        = mean_reversion(price, fair_value)
shock        = event_impact(state_deltas)   // §5.5
noise        = seeded_random_walk(volatility_of_class)
impact       = accumulated_order_flow       // the player's own trades
price'       = price + drift + shock + noise + impact
```

Skill is estimating `fair_value` faster and better than the market's drift can close the
gap. Every one of those five terms is inspectable on the listing screen after the fact —
Pillar 1 requires that a player can always ask "why did it move" and get a decomposition,
not a shrug.

### 5.5 Event Impact — read deltas, not prose

**A deliberate architectural decision, and the one most likely to be got wrong.**
Mytherra's `WorldEvent` is `{ year, kind, message }` where `kind` has only four coarse
variants (Divine, Region, Hero, System) and the substance is free text. Parsing that
prose for market signals would be fragile and would silently break whenever a string is
reworded.

So: **the price oracle reads numeric state deltas between consecutive `WorldView`s.** A
region's `harvest` fell 30 points and `famine` flipped true; a house's `member_ids` lost
its highest-renown hero; a trade route's `volume` halved; a resource node's status
changed. These are typed, stable, and testable.

The chronicle is rendered *alongside* as the newspaper — authored prose that explains, in
the world's voice, what the numbers just did. Pillar 4 falls directly out of this split:
the prose is flavour and the deltas are truth, and a player who trades on headlines alone
is trading on a lagging indicator.

### 5.6 The Settlement Clock and Eras

One settlement = one world tick = one year. The original's
`TimeSimulationParams { days, speed }` becomes: settle one year, settle a decade
(auto-resolving positions and reporting a digest), or settle to the next flagged event.

**Eras are the campaign's acts.** Mytherra runs hundred-year eras that end in a cataclysm.
The exchange should make an approaching era-end visible and *unpriceable* — the player
knows roughly when, never what. Surviving a cataclysm solvent is the game's signature
achievement, and an Archivist licence (§5.3) letting a player backtest previous
cataclysms is the reward for the deepest information tier.

### 5.7 Why this is not Mytherra's Observatory

Mytherra already has a Divine Observatory betting layer with a `BettingMarket` enum
(HeroFate, RegionFortune, WorldTurning, RegionCollapse). The distinction must stay sharp:

| | Observatory (Mytherra) | Exchange (Tradeborn Realms) |
| --- | --- | --- |
| Instrument | Binary wager on a predicate | Continuous position on an entity |
| Exit | None — it resolves | Any time, at a price, with market impact |
| Held by | A deity who can also *cause* the outcome | A broker who cannot influence anything |
| Skill | Predicting a discrete event | Valuing an ongoing claim, and sizing the position |

They are complementary, and the asymmetry is thematically pointed: the gods bet on
outcomes they can rig, while the mortals downstream trade paper on the consequences.

### 5.8 Randomness & Determinism

Two seeded `macroquad_toolkit::rng` streams, kept separate so they cannot desynchronise
each other: one owned by the world sim (in Solo mode; absent in Live), one owned by the
market for noise and order-flow effects. Both seeds are saved and displayed on the records
screen. Same seeds + same orders → identical price history, which is what makes the
§14 backtest harness possible.

---

## 6. Data Model (`assets/*.json`)

The world model comes from `mytherra-core`; these files define the *market* over it.

| File | Defines | Loaded via |
| --- | --- | --- |
| `assets/data/game_config.json` | Starting capital, fee schedule, settlement length, margin rules, licence costs | `load_embedded_json_labeled` |
| `assets/instruments.json` | The taxonomy of §5.2: per class, how to derive `fair_value` from fundamentals, base volatility, liquidity, income rule | `DataRegistry` |
| `assets/impacts.json` | State-delta → price-shock table (§5.5): which field changing by how much moves which instrument class, and by how much | `DataRegistry` |
| `assets/licences.json` | Information tiers (§5.3), costs, the `VisibilityScope` each maps to | `DataRegistry` |
| `assets/achievements.json` | Achievement definitions | `achievements` |
| `assets/strings.json` | UI copy, ticker abbreviations, broker-desk flavour | `DataRegistry` |
| `assets/world_seed.json` | Solo-mode world seed / recorded replay reference | `mytherra-core` loaders |

`impacts.json` is the balance surface of the entire game and must never be Rust
constants — it is where a designer tunes how hard a famine hits grain.

---

## 7. World & Progression Structure

- **World layout:** No spatial world. Mytherra's regions and settlements appear as
  *listings and research pages*, never as a map. (If a map is ever wanted, that is
  Mytherra's job, not this game's.)
- **Session length:** a settlement is 1–3 minutes; an era is ~100 settlements but
  decade-stepping compresses quiet stretches. A campaign is one era (3–6 hours) or a
  multi-era dynasty run.
- **Progression:** capital, licences (§5.3), unlocked instrument classes, leverage access,
  and the records archive. Licences are the meaningful axis.
- **Save model:** `save_to_slot_with_version` / `load_from_slot_with_migration`. Saved:
  both seeds, world state or replay cursor, tick number, cash, positions with cost basis,
  order history, watchlist, licences, achievements, and the price history series (via
  `series`, which is bounded by construction — the reason that module exists).

---

## 8. Content Inventory

| Content type | In old game | Prototype target | Full target |
| --- | ---: | ---: | ---: |
| **Tradeable instruments** | **5 (static)** | 40 | 120+ (emergent — the world generates them) |
| Instrument classes | 1 | 4 | 7 |
| Fundamental → fair-value rules | 0 | 4 | 7 |
| State-delta impact rules | 0 | 25 | 80 |
| Information licences | 0 | 4 | 4 |
| Achievements | ~10 | 15 | 45 |
| Broker-desk flavour lines | 0 | 60 | 200 |
| Chronicle prose | 0 | — | supplied by Mytherra |

The instrument count is the headline: the world *generates* listings. Houses are founded
and die out, settlements are established and razed, resource nodes are discovered. A
listing being **delisted because the thing it represented no longer exists** is content no
authored market can produce.

---

## 9. UI/UX & Screen Flow

UI is a pure view layer returning `UiAction`; an `exchange_actions.rs` dispatcher applies
them.

| Screen | Purpose | Toolkit pieces |
| --- | --- | --- |
| **Floor** | The board: all listings, price, change, volume, filters, sort | `GridLayout`, scroll, `TextStyle`, `colors` |
| **Listing** | One instrument: chart, fundamentals, the underlying entity, price-move decomposition | **`series`** + `ui` primitives, meters, badges |
| **Order ticket** | Buy/sell, size, estimated market impact, fees, resulting exposure | Form surface |
| **Portfolio** | Positions, cost basis, P&L, exposure by class and region, margin | `GridLayout`, meters |
| **Chronicle** | Mytherra's event feed, filterable by kind, cross-linked to affected listings | Scroll list |
| **Research** | Entity browser gated by licence — regions, houses, orders, heroes | Scroll list, tooltips |
| **Records** | Equity curve, trade history, seeds, era archive | `series` |
| Pause / Settings | | `settings` |

**The chart is the game's one piece of real visual craft.** It must handle a century of
data without losing the spike that mattered — which is precisely what `series.rs` was
built to do ("never grows and never loses an extreme"). Candles or line, an era-boundary
rule-off, and the ability to overlay a fundamental against the price so the player can see
the gap they are trading.

Flow per settlement: floor → listing → order ticket → (repeat) → settle → digest →
chronicle → floor.

---

## 10. Toolkit Mapping

| Need | Toolkit module | Using it? | Notes |
| --- | --- | --- | --- |
| Input handling | `input` | Yes | |
| Widgets/layout/text | `ui` | **Yes — the bulk** | Dense tabular layout is the main demand |
| **Time series / charts** | **`series`** | **Yes — the headline use** | Second adopter after `dragons_hoard`; expect to push improvements back |
| Textures/manifest | `assets` | No | No textures |
| Camera/pan/zoom | `camera` | No | Chart zoom is a data range, not a camera |
| Cross-system messaging | `events` | Yes | `EventBus<UiAction>` |
| Palette | `colors` | Yes | Up/down/flat semantics — must be colour-blind safe, see §12 |
| Vector/grid math | `math` | Yes | Chart scaling |
| Frame timing | `timing` | Yes | Settlement pacing, ticker animation |
| Particles/juice | `fx` | Sparingly | A crash deserves weight; restraint elsewhere |
| User settings | `settings` | Yes | |
| Unlocks/achievements | `achievements` | Yes | |
| Dev overlay | `debug` | Yes | Show the §5.4 decomposition live |
| Deterministic randomness | `rng` | **Yes — critical** | Two separate streams, §5.8 |
| Sprite animation | `sprite` | No | |
| Procedural images | `raster` / `paint` | Maybe | If chart rendering wants golden-image tests, `paint` is the route |
| Headless capture | `capture` | Yes (required) | `TB_REALMS_CAPTURE_*`, already wired |
| Save/load | `persistence` | Yes | |
| Tile grid / fog / pathing | `FlatGrid`, `FogState` | No | **Strip the template's grid/fog scaffolding** |
| **Cross-platform HTTP** | — | Live mode only | `quad-net`, following `mytherra/src/net.rs`'s poll-based `ServerClient` exactly. Do not invent a second HTTP pattern |

Likely toolkit upgrades to raise in `rust_management` rather than solve locally: chart
axis/tick labelling, and a sortable-table widget. Both would serve other games.

---

## 11. Architecture Skeleton

```
src/
├── main.rs
├── game.rs                  # Game struct, state machine, transition()
├── game/
│   ├── states.rs            # Menu, Floor, Listing, Settling, Digest, Records
│   └── capture_scenes.rs
├── world.rs                 # THE SEAM: one trait, two impls, one WorldView out
├── world/
│   ├── solo.rs              # embedded mytherra-core, ticked locally
│   ├── live.rs              # quad-net ServerClient against mytherra-server
│   └── delta.rs             # typed diff between consecutive WorldViews (§5.5)
├── market.rs                # instruments, fair value, price formation, order book
├── market/
│   ├── instruments.rs       # the §5.2 taxonomy and fair_value rules
│   ├── pricing.rs           # the §5.4 five-term model
│   ├── impact.rs            # state-delta → shock, driven by impacts.json
│   └── orders.rs            # execution, fees, market impact
├── data.rs                  # embedded JSON + registries
├── data/
├── state.rs                 # GameSession, portfolio, SaveData, migration
├── state/
├── ui.rs
├── ui/                      # floor, listing, chart, ticket, portfolio, chronicle,
│                            # research, records
└── exchange_actions.rs      # UiAction dispatcher
```

`world.rs` is the most important file in the project. One trait — *give me the current
`WorldView` and the events since cursor N* — with a local implementation and a networked
one. Everything above it is mode-agnostic. Get this seam right in M0 and Live mode is a
later afternoon; get it wrong and the modes fork the whole codebase.

`market/` must not depend on macroquad, so the backtest harness (§14) can run headless.

---

## 12. Non-Goals / Open Questions

**Non-goals:** real money, purchasable currency, or anything that reads as gambling with
value at stake; PvP or a shared order book; **any write path into Mytherra** — no
`PlayerAction`, no nudges, no divine favour spent from this game; a rendered world map;
real-world financial instruments or terminology beyond what the fantasy needs.

**Open questions:**

1. **Does Solo mode embed `mytherra-core` or replay recorded snapshots?** The
   cross-workspace dependency is confirmed to work (§5.1), so this is now purely a
   WASM build-size and tick-cost question rather than a feasibility one. Embedding gives a
   genuinely live world and infinite replayability; replay makes WebGL builds smaller and
   startup faster. Measure in M0.
2. **Is a hundred-year era too long for one sitting?** Decade-stepping may be enough, or
   the campaign may want to be a *quarter*-era with cataclysms as a New Game+ reward.
3. **How much does the player's own order flow move the market?** Too little and capital
   has no ceiling; too much and the game becomes about hiding your size. This is the
   single hardest balance question in the design.
4. **Should Live mode be tradeable at all, or read-only spectating?** A shared world
   whose deities can rig outcomes is a market with insider trading by construction. That
   is either the best or worst idea in this document.
5. **Up/down colour semantics.** Red/green is the convention and the worst possible choice
   for colour-blind players. `dragons_hoard` already solved an equivalent problem
   (§5.24/§5.41 of its GDD — "reading the symbols without colour"); steal that approach
   rather than re-deriving it.
6. Does a "delisted" instrument stay in the records archive forever? It should — a house
   that died out in 340 is exactly what an Archivist licence is for.

---

## 13. Milestones

| Milestone | Contents | Done when |
| --- | --- | --- |
| **M0 — The seam** | Re-add the mytherra path deps (verified working, §5.1), `world.rs` trait, Solo impl over `mytherra-core`, `WorldView` delta computation, save round-trip. **Measure the WASM cost to resolve open question 1.** | A world can be ticked 100 years headlessly and typed deltas emitted per tick |
| **M1 — Instruments** | The §5.2 taxonomy, `instruments.json`, fair-value rules, listings generated from a live `WorldView` | Listings appear, disappear, and revalue as the world runs — with no market yet |
| **M2 — The market** | §5.4 price formation, `impacts.json`, order execution, fees, market impact, portfolio and P&L | A full century can be traded headlessly and the equity curve is plausible |
| **M3 — The floor** | Floor, listing, chart via `series`, order ticket, portfolio screens | A settlement is playable end to end |
| **M4 — The world made legible** | Chronicle feed, research browser, price-move decomposition, cross-links | Every price move can be traced to a delta and a chronicle line |
| **M5 — Licences and eras** | §5.3 information tiers gated on `VisibilityScope`, era transitions, cataclysm handling, records archive | A cataclysm is survivable and visibly ruinous |
| **M6 — Live mode** | `world/live.rs` via `quad-net`, session handling, offline/reconnect, mode parity tests | Live and Solo produce identical behaviour given identical `WorldView`s |
| **M7 — Polish** | Achievements, accessibility (open question 5, text size, keyboard nav), audio via `synth`, balance pass on order-flow impact | `publish.ps1` clean; CI green; capture set covers every screen |

M6 is explicitly last. The game must be complete and shippable without a server ever
running.

---

## 14. Verification

- **Backtest harness** — the headline test. `market/` is macroquad-free, so CI can run a
  full century with a scripted strategy and assert the equity curve stays inside designed
  bounds. This catches balance regressions that no screenshot can.
- **Determinism test** — same two seeds + same order sequence → identical price history
  and identical portfolio.
- **Mode parity test** — feed `world/solo.rs` and `world/live.rs` the same recorded
  `WorldView` sequence; every downstream number must be bit-identical. This is what keeps
  the §5.1 promise honest.
- **Delta coverage test** — every field the §5.5 impact table references must exist on
  `WorldView` and be exercised by at least one fixture. This is the test that fails loudly
  if Mytherra changes its model, which is the main risk of the tie-in.
- **No-orphan-instrument test** — every instrument class must be derivable from at least
  one entity in a freshly generated world.
- **Screenshot capture** per screen via `scripts/capture_ui.ps1`.
