//! The market end to end: a century of settlements, headlessly (GDD 14).
//!
//! Nothing under `market/` depends on macroquad, which is what lets this run at
//! all. The century here drives a *synthetic* evolution of a real seeded world —
//! prosperity wanders, famines come and go, mines work out — so the whole chain
//! is exercised without a server: read the world, diff it, revalue the board,
//! shock it, settle every price.

use crate::data::GameData;
use crate::market::{Board, Instrument};
use crate::world::delta::diff;
use crate::world::fundamentals::EntityRef;
use crate::world::Snapshot;
use macroquad_toolkit::rng::SeededRng;
use mytherra_core::data::ResourceStatus;
use mytherra_core::world::WorldState;
use mytherra_protocol::{project_world, spectator_standing, WorldView};
use std::collections::BTreeMap;

fn base() -> WorldView {
    let content = mytherra_core::data::GameData::load().expect("mytherra content loads");
    project_world(&WorldState::new(&content), &spectator_standing())
}

/// One year of a plausible world: stats wander, granaries fill and empty,
/// famine comes and goes, and now and then a mine is worked out.
///
/// Deliberately its own RNG stream, seeded apart from the market's, so a change
/// to how prices are drawn cannot silently change the history being priced.
fn turn(view: &WorldView, rng: &mut SeededRng) -> WorldView {
    let mut next = view.clone();
    next.year += 1;
    next.tick_count += 1;

    for region in &mut next.regions {
        region.prosperity = (region.prosperity + rng.range_f32(-4.0, 4.0)).clamp(0.0, 100.0);
        region.chaos = (region.chaos + rng.range_f32(-5.0, 5.0)).clamp(0.0, 100.0);
        region.harvest = (region.harvest + rng.range_f32(-9.0, 8.0)).clamp(0.0, 100.0);
        region.famine = region.harvest < 25.0;
    }
    for settlement in &mut next.settlements {
        settlement.population = (settlement.population * rng.range_f32(0.97, 1.04)).max(0.0);
        settlement.prosperity =
            (settlement.prosperity + rng.range_f32(-3.0, 3.0)).clamp(0.0, 100.0);
    }
    if rng.chance(0.15) {
        let node = rng.below(next.resource_nodes.len());
        next.resource_nodes[node].status = ResourceStatus::Depleted;
    }
    next
}

/// Run `years` of settlements and return every listing's final price by ticker.
fn run_century(years: u32, world_seed: u64) -> BTreeMap<String, f32> {
    let data = GameData::load().expect("the exchange's data loads");
    let market = &data.config.market;
    let mut world_rng = SeededRng::new(world_seed);
    let mut market_rng = SeededRng::new(market.seed);

    let mut view = base();
    let mut board = Board::new();
    board.refresh(&Snapshot::read(&view), &data.instruments, market);

    let no_flow = BTreeMap::new();
    for _ in 0..years {
        let next = turn(&view, &mut world_rng);
        let snapshot = Snapshot::read(&next);
        let delta = diff(&view, &next);

        board.refresh(&snapshot, &data.instruments, market);
        let shocks = data.impacts.shocks(&delta, &snapshot);
        board.settle(&shocks, &no_flow, market, &mut market_rng);

        view = next;
    }

    board
        .listings()
        .map(|listing| (listing.symbol.clone(), listing.price))
        .collect()
}

#[test]
fn a_century_of_settlements_runs_headlessly_and_leaves_every_price_sane() {
    let data = GameData::load().unwrap();
    let market = &data.config.market;
    let mut world_rng = SeededRng::new(4242);
    let mut market_rng = SeededRng::new(market.seed);

    let mut view = base();
    let mut board = Board::new();
    board.refresh(&Snapshot::read(&view), &data.instruments, market);
    let opening = board.len();
    assert!(opening > 40, "a world of substance opens the board");

    let no_flow = BTreeMap::new();
    let mut settlements_that_moved = 0;
    for _ in 0..100 {
        let next = turn(&view, &mut world_rng);
        let snapshot = Snapshot::read(&next);
        let delta = diff(&view, &next);
        board.refresh(&snapshot, &data.instruments, market);
        board.settle(
            &data.impacts.shocks(&delta, &snapshot),
            &no_flow,
            market,
            &mut market_rng,
        );
        if board.listings().any(|l| l.last_move.change() != 0.0) {
            settlements_that_moved += 1;
        }
        view = next;
    }

    assert!(
        settlements_that_moved > 90,
        "a living world should move prices nearly every year, moved {settlements_that_moved}"
    );

    for listing in board.listings() {
        assert!(
            listing.price >= market.min_price,
            "{} fell through the floor at {}",
            listing.symbol,
            listing.price
        );
        assert!(
            listing.price.is_finite(),
            "{} is not a number",
            listing.symbol
        );
        // A century of mean reversion should leave price and fair value in the
        // same postcode. Wide, because it is a bound on runaway, not a balance
        // assertion — but a price that had drifted an order of magnitude away
        // would mean drift is not working at all.
        assert!(
            listing.price < listing.fair_value.max(market.min_price) * 6.0 + 20.0,
            "{} ran away: price {} against fair {}",
            listing.symbol,
            listing.price,
            listing.fair_value
        );
        // And the history kept up, without growing without bound.
        assert!(!listing.history.is_empty());
        assert!(listing.history.len() <= market.history_capacity + 2);
    }
}

#[test]
fn the_same_seeds_replay_the_same_century() {
    // GDD 14's determinism test, at the level the whole game runs at: same world
    // history, same market stream, same prices to the last decimal.
    let once = run_century(60, 909);
    let twice = run_century(60, 909);
    assert_eq!(once, twice, "the century is not reproducible");

    // A different world is a different century, which is the other half of the
    // claim — otherwise "deterministic" could just mean "ignores its inputs".
    let elsewhere = run_century(60, 910);
    assert_ne!(once, elsewhere);
}

#[test]
fn the_history_keeps_the_spike_that_mattered() {
    // The whole reason `series` exists: a century decimated down to a bounded
    // buffer must still remember its extremes. A broker looking back at a
    // cataclysm needs to see the crater, not a smoothed shrug.
    let data = GameData::load().unwrap();
    let market = &data.config.market;
    let mut market_rng = SeededRng::new(market.seed);
    let mut world_rng = SeededRng::new(77);

    let mut view = base();
    let mut board = Board::new();
    board.refresh(&Snapshot::read(&view), &data.instruments, market);

    let no_flow = BTreeMap::new();
    let mut peaks: BTreeMap<String, f32> = BTreeMap::new();
    for _ in 0..300 {
        let next = turn(&view, &mut world_rng);
        let snapshot = Snapshot::read(&next);
        let delta = diff(&view, &next);
        board.refresh(&snapshot, &data.instruments, market);
        board.settle(
            &data.impacts.shocks(&delta, &snapshot),
            &no_flow,
            market,
            &mut market_rng,
        );
        for listing in board.listings() {
            let peak = peaks.entry(listing.symbol.clone()).or_insert(listing.price);
            *peak = peak.max(listing.price);
        }
        view = next;
    }

    // Three hundred settlements into a buffer of 256 means the series has been
    // decimated at least once, so this is a real test of the merge and not of
    // an unfilled buffer.
    assert!(
        300 > market.history_capacity,
        "the series must have decimated"
    );

    let mut checked = 0;
    for listing in board.listings() {
        let peak = peaks[&listing.symbol];
        let remembered = listing.history.max().expect("a listing has a history");
        assert!(
            remembered >= peak - 0.01,
            "{} forgot its high: remembers {remembered}, reached {peak}",
            listing.symbol
        );
        checked += 1;
    }
    assert!(checked > 40, "the fixture must actually check something");
}

/// The whole chain against the running world — the one test that would catch
/// the exchange pricing a world it is not actually reading.
///
/// Waits out a real server tick, so it is `#[ignore]`d with the other
/// live-server tests and run deliberately.
#[test]
#[ignore = "needs a live mytherra-server and waits out one of its ticks"]
fn the_live_world_moves_prices_when_it_turns() {
    use crate::world::live::{LiveWorld, SpectatorClient};
    use crate::world::WorldSource;

    let data = GameData::load().unwrap();
    let market = &data.config.market;
    let token = data
        .config
        .spectator_token()
        .expect("set TB_REALMS_SPECTATOR_TOKEN to run this");
    let mut world = LiveWorld::connect(
        SpectatorClient::new(data.config.world_url(), token),
        0.25,
        15.0,
    );
    let mut board = Board::new();
    let mut market_rng = SeededRng::new(market.seed);
    let no_flow = BTreeMap::new();

    // Poll until the world turns, at a simulated quarter-second a step.
    let mut turning = None;
    for _ in 0..2_000 {
        if let Some(delta) = world.poll(0.25) {
            turning = Some(delta);
            break;
        }
        if let Some(view) = world.view() {
            board.refresh(&Snapshot::read(view), &data.instruments, market);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let delta = turning.expect("the world did not turn within the timeout");
    let view = world.view().expect("a world is in hand");

    let snapshot = Snapshot::read(view);
    board.refresh(&snapshot, &data.instruments, market);
    let shocks = data.impacts.shocks(&delta, &snapshot);
    board.settle(&shocks, &no_flow, market, &mut market_rng);

    println!(
        "year {}: {} listings, {} shocked",
        delta.to_year,
        board.len(),
        shocks.len()
    );
    let mut movers: Vec<&Instrument> = board
        .listings()
        .filter(|l| l.last_move.change() != 0.0)
        .collect();
    movers.sort_by(|a, b| {
        b.last_move
            .change_fraction()
            .abs()
            .total_cmp(&a.last_move.change_fraction().abs())
    });
    for listing in movers.iter().take(6) {
        println!(
            "  {:6} {:24} {:8.2} {:+6.2}%   {}",
            listing.symbol,
            listing.name,
            listing.price,
            listing.last_move.change_fraction() * 100.0,
            listing
                .last_move
                .parts()
                .into_iter()
                .map(|(name, value)| format!("{name} {value:+.2}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    assert!(board.len() > 20, "the live world backs a real board");
    assert!(!movers.is_empty(), "a year passed and no price moved");
    // Every mover's decomposition must add up, on real data and not just on
    // fixtures — this is the assertion that would catch a term going missing.
    for listing in &movers {
        let moved = listing.last_move;
        let summed = moved.drift + moved.shock + moved.noise + moved.flow;
        assert!(
            (summed - moved.change()).abs() < 0.01,
            "{} does not add up: {moved:?}",
            listing.symbol
        );
    }
}

/// A scripted broker: buy the cheapest paper on the board, sell anything that
/// has run well past what the world says it is worth.
///
/// Deliberately naive. The point is not that it is a good strategy — it is that
/// a strategy trading through the real execution path for a century produces an
/// equity curve inside designed bounds. A balance change that made trading free,
/// or made every position a loser, moves this.
fn backtest(years: u32, world_seed: u64) -> Vec<f32> {
    use crate::market::orders::Side;
    use crate::state::Portfolio;

    let data = GameData::load().expect("the exchange's data loads");
    let (market, orders) = (&data.config.market, &data.config.orders);
    let mut world_rng = SeededRng::new(world_seed);
    let mut market_rng = SeededRng::new(market.seed);

    let mut view = base();
    let mut board = Board::new();
    let mut ledger = Portfolio::new(orders);
    board.refresh(&Snapshot::read(&view), &data.instruments, market);

    let mut flow: BTreeMap<EntityRef, f32> = BTreeMap::new();
    let mut curve = Vec::new();

    for year in 0..years {
        let next = turn(&view, &mut world_rng);
        let snapshot = Snapshot::read(&next);
        let delta = diff(&view, &next);
        board.refresh(&snapshot, &data.instruments, market);
        board.settle(
            &data.impacts.shocks(&delta, &snapshot),
            &flow,
            market,
            &mut market_rng,
        );
        for mark in flow.values_mut() {
            *mark *= orders.flow_decay;
        }
        ledger.collect_income(&board);
        ledger.write_off_delisted(&board);

        // Sell what has run past fair value.
        let dear: Vec<Instrument> = ledger
            .positions()
            .filter_map(|position| board.get(&position.entity))
            .filter(|listing| listing.premium() > 0.20)
            .cloned()
            .collect();
        for listing in dear {
            let held = ledger.units_of(&listing.entity);
            if let Ok(fill) = ledger.execute(&listing, Side::Sell, held, next.year, orders) {
                *flow.entry(listing.entity.clone()).or_default() += fill.flow;
            }
        }

        // Buy the cheapest thing the ledger can afford a round lot of.
        let mut cheap: Vec<&Instrument> = board
            .listings()
            .filter(|listing| listing.premium() < -0.10)
            .collect();
        cheap.sort_by(|a, b| a.premium().total_cmp(&b.premium()));
        if let Some(listing) = cheap.first().map(|l| (*l).clone()) {
            let affordable = (ledger.cash * 0.25 / listing.price.max(0.01)) as i64;
            if affordable > 0 {
                if let Ok(fill) = ledger.execute(&listing, Side::Buy, affordable, next.year, orders)
                {
                    *flow.entry(listing.entity.clone()).or_default() += fill.flow;
                }
            }
        }

        curve.push(ledger.equity(&board));
        view = next;
        let _ = year;
    }
    curve
}

#[test]
fn a_century_can_be_traded_headlessly_and_the_equity_curve_is_plausible() {
    // GDD 14's headline test, and M2's "done when". `market/` is macroquad-free
    // precisely so this can run in CI.
    let opening = GameData::load().unwrap().config.orders.starting_capital;
    let curve = backtest(100, 31_337);

    assert_eq!(curve.len(), 100);
    for (year, equity) in curve.iter().enumerate() {
        assert!(
            equity.is_finite() && *equity > 0.0,
            "year {year}: equity is {equity}"
        );
        // Bounds, not balance: a broker who cannot lose everything and cannot
        // multiply their capital a hundredfold in a century is inside a market
        // that behaves like one. Either bound breaking means the price model or
        // the fee model has stopped biting.
        assert!(
            *equity < opening * 100.0,
            "year {year}: equity ran away to {equity}"
        );
    }

    let ended = *curve.last().unwrap();
    println!("backtest: opened {opening:.0}, ended {ended:.0}");
    // The curve must actually move — a strategy that never traded would sit
    // flat at the opening capital and pass every bound above.
    assert!(
        (ended - opening).abs() > opening * 0.01,
        "the scripted broker did nothing: {ended} against {opening}"
    );
}

#[test]
fn the_same_seeds_replay_the_same_traded_century() {
    // Determinism through the execution path too, not just the price path:
    // same world, same market stream, same orders, same equity curve.
    assert_eq!(backtest(50, 2024), backtest(50, 2024));
    assert_ne!(backtest(50, 2024), backtest(50, 2025));
}
