//! Live-source tests.
//!
//! The unit tests here cover what can be checked without a server. The
//! end-to-end test is `#[ignore]`d and needs a running `mytherra-server`,
//! following the precedent in `mytherra/src/net.rs`:
//!
//! ```powershell
//! cargo test -- --ignored live_world
//! ```

use super::*;

#[test]
fn the_live_source_issues_only_reads() {
    // A guard on GDD Pillar 2 — the exchange must never learn to write. Every
    // mutating call into Mytherra would have to leave this file as a non-GET
    // request, and quad-net's builder defaults to GET, so a write is impossible
    // without naming a method here.
    //
    // It is a source-text check, which is a blunt instrument: it proves this
    // module sends no write, not that no write exists anywhere. It is cheap,
    // and it fails loudly the moment someone reaches for `Method::Post` in the
    // one file that talks to the world.
    let source = include_str!("../live.rs");
    for method in [
        "Method::Post",
        "Method::Put",
        "Method::Delete",
        "Method::Patch",
    ] {
        assert!(
            !source.contains(method),
            "the exchange is downstream of the world: found {method} in the live source"
        );
    }
    assert!(
        source.contains("/spectate"),
        "the live source reads the spectator endpoints"
    );
}

#[test]
fn poll_intervals_and_timeouts_are_floored() {
    // A zero or negative interval would issue a request every frame and flood
    // the server; a zero timeout would fail every request instantly.
    let world = LiveWorld::connect(SpectatorClient::new("http://127.0.0.1:1", "t"), 0.0, 0.0);
    assert!(world.poll_seconds >= 0.1);
    assert!(world.timeout_seconds >= 1.0);
}

#[test]
fn a_fresh_source_is_connecting_with_no_world_and_a_zero_cursor() {
    let world = LiveWorld::connect(SpectatorClient::new("http://127.0.0.1:1", "t"), 2.0, 5.0);
    assert_eq!(world.status(), &SourceStatus::Connecting);
    assert!(world.view().is_none());
    assert_eq!(world.cursor(), 0);
}

#[test]
fn an_unreachable_server_fails_the_status_without_hanging() {
    // Port 1 refuses immediately on native. The timeout is the backstop that
    // matters on wasm, where a refusal never resolves on its own.
    let mut world = LiveWorld::connect(SpectatorClient::new("http://127.0.0.1:1", "t"), 0.5, 1.0);

    // Drive frames until the source gives up, with a hard cap so a regression
    // fails the test rather than hanging it.
    let mut frames = 0;
    while matches!(world.status(), SourceStatus::Connecting) && frames < 600 {
        assert!(
            world.poll(0.1).is_none(),
            "a failed read must not emit a settlement"
        );
        frames += 1;
    }

    match world.status() {
        SourceStatus::Failed(err) => assert!(!err.is_empty()),
        other => panic!("expected the wire to be reported down, got {other:?}"),
    }
    // The banner explains itself rather than showing a bare state name.
    assert!(world.status().label().starts_with("wire down"));
    assert!(world.view().is_none(), "no world was ever received");
}

/// End-to-end against a running `mytherra-server`. Start it with
/// `cargo run -p mytherra-server` from `../mytherra`, then run:
/// `cargo test -- --ignored live_world`.
#[test]
#[ignore = "needs a live mytherra-server on 127.0.0.1:8791 with SPECTATOR_TOKEN set"]
fn live_world_reads_the_shared_world_and_prices_nothing_it_cannot_see() {
    let token = std::env::var("TB_REALMS_SPECTATOR_TOKEN")
        .expect("set TB_REALMS_SPECTATOR_TOKEN to the server's SPECTATOR_TOKEN");
    let mut world = LiveWorld::connect(
        SpectatorClient::new("http://127.0.0.1:8791", token),
        1.0,
        10.0,
    );

    // Poll until the first world lands.
    let mut frames = 0;
    while world.view().is_none() && frames < 2000 {
        world.poll(0.016);
        std::thread::sleep(std::time::Duration::from_millis(5));
        frames += 1;
    }

    let view = world.view().expect("a world arrived");
    assert_eq!(world.status(), &SourceStatus::Live);
    // A spectator sees every class the instrument taxonomy is built on (GDD 5.2)
    // — this is the assertion that fails if the server's spectator Standing is
    // ever narrowed.
    assert!(!view.regions.is_empty(), "regional bonds need regions");
    assert!(!view.settlements.is_empty());
    assert!(!view.trade_routes.is_empty());
    assert!(!view.resource_nodes.is_empty());
    assert!(!view.heroes.is_empty());
    // And no wagers, because a broker is not a deity (GDD 5.7).
    assert!(view.speculations.is_empty());
}

/// The M0 acceptance test (GDD 13): the world turns and the exchange emits a
/// typed delta for it. Waits out at least one of the server's own ticks, so it
/// is slow by nature — the world's clock is not ours to hurry, which is the
/// whole point. Run with the server up:
/// `cargo test -- --ignored --nocapture a_turning_of_the_world`.
#[test]
#[ignore = "needs a live mytherra-server and waits out one of its ticks"]
fn a_turning_of_the_world_arrives_as_a_typed_delta() {
    let token = std::env::var("TB_REALMS_SPECTATOR_TOKEN")
        .expect("set TB_REALMS_SPECTATOR_TOKEN to the server's SPECTATOR_TOKEN");
    let mut world = LiveWorld::connect(
        SpectatorClient::new("http://127.0.0.1:8791", token),
        1.0,
        10.0,
    );

    // The server's default cadence is 10s/tick; allow generous slack for a
    // slower configured interval before giving up.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    let delta = loop {
        if let Some(delta) = world.poll(0.05) {
            break delta;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no turning observed before the deadline (status: {:?})",
            world.status()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    println!(
        "year {} (+{}y): {} moves, {} listed, {} delisted, {} chronicle lines",
        delta.to_year,
        delta.years(),
        delta.moves.len(),
        delta.listed.len(),
        delta.delisted.len(),
        delta.events.len()
    );
    for moved in delta.most_moved(5) {
        println!(
            "  {} {} {}",
            moved.entity.class(),
            moved.entity.id(),
            moved.field.key()
        );
    }

    assert!(delta.to_year >= 1, "the delta names the world's year");
    assert!(delta.to_tick > 0);
    // A living world that ran a whole year and moved nothing at all would mean
    // the diff is reading the wrong fields — the failure this test exists for.
    assert!(
        !delta.is_empty(),
        "a year passed in a living world and nothing was detected as moving"
    );
    // The chronicle cursor advances, so the next read is not served this prose
    // a second time.
    assert!(world.cursor() > 0, "the chronicle cursor advanced");
}
