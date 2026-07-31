//! The session: the log, the chronicle, the board, and the save.

use super::*;
use crate::data::GameData;
use crate::world::delta::WorldDelta;
use mytherra_protocol::{project_world, spectator_standing, WorldView};

fn data() -> GameData {
    GameData::load().expect("the exchange's data loads")
}

fn market() -> MarketConfig {
    data().config.market
}

fn orders() -> OrderConfig {
    data().config.orders
}

/// A freshly seeded world as a spectator receives it.
fn world() -> WorldView {
    let content = mytherra_core::data::GameData::load().expect("mytherra content loads");
    project_world(
        &mytherra_core::world::WorldState::new(&content),
        &spectator_standing(),
    )
}

fn delta(year: u32, tick: u64) -> WorldDelta {
    WorldDelta {
        to_tick: tick,
        from_year: year.saturating_sub(1),
        to_year: year,
        ..Default::default()
    }
}

#[test]
fn settlements_are_logged_newest_first_and_bounded() {
    let data = data();
    let mut session = GameSession::new(&market(), &orders());
    // An empty reading of the world: this test is about the log, and an
    // empty board keeps the settling out of its way.
    for year in 1..=(LOG_CAPACITY as u32 + 10) {
        session.observe(
            Snapshot::default(),
            Some(&delta(year, year as u64)),
            1,
            &data,
        );
    }

    assert_eq!(
        session.settlements().len(),
        LOG_CAPACITY,
        "the log is bounded"
    );
    assert_eq!(
        session.settlements()[0].year,
        LOG_CAPACITY as u32 + 10,
        "newest first"
    );
    // The count outlives the buffer, so the digest can say how long the
    // exchange has been open even after the log has rolled.
    assert_eq!(session.observed, LOG_CAPACITY as u64 + 10);
    assert_eq!(session.last_tick, LOG_CAPACITY as u64 + 10);
}

#[test]
fn a_save_round_trips_the_cursor_and_tick() {
    let mut session = GameSession::new(&market(), &orders());
    session.observe(Snapshot::default(), Some(&delta(341, 17)), 1, &data());

    let save = session.to_save("0.1.0", 4_212);
    let resumed = GameSession::from_save(&save, &market(), &orders());

    assert_eq!(resumed.last_tick, 17);
    // Neither the log nor the board is persisted — the log is a display
    // buffer and the board is a function of the world, and the next read
    // repopulates both.
    assert!(resumed.settlements().is_empty());
    assert!(resumed.board().is_empty());
}

#[test]
fn the_board_hangs_from_the_world_and_the_digest_names_the_tickers() {
    let data = data();
    let view = world();

    let mut session = GameSession::new(&market(), &orders());
    let opening = session.observe(Snapshot::read(&view), None, 1, &data).board;

    assert!(!session.board().is_empty(), "the world hangs the board");
    assert_eq!(opening.listed.len(), session.board().len());
    assert!(opening.delisted.is_empty());
    // The reading is kept, so the listing screen can show the fundamentals
    // a price was derived from without asking the world again.
    assert!(session.snapshot().iter().count() > 0);

    // A settlement's digest reports the board's own tickers, so a razed
    // town reads as paper struck off rather than as an entity count.
    let mut razed = view.clone();
    let lost = razed.settlements.remove(0);
    let turning = delta(razed.year + 1, 2);
    let change = session
        .observe(Snapshot::read(&razed), Some(&turning), 1, &data)
        .board;

    assert_eq!(change.delisted.len(), 1);
    assert_eq!(session.settlements()[0].delisted, 1);
    assert!(
        session.settlements()[0].notable[0].ends_with("delisted"),
        "{:?}",
        session.settlements()[0].notable
    );
    assert!(
        session
            .board()
            .listings()
            .all(|listing| listing.name != lost.name),
        "the razed town is off the board"
    );
}

#[test]
fn the_exchange_opens_with_the_worlds_back_issues_already_on_the_desk() {
    // The world has a past whether or not this session watched it happen,
    // and the first view carries it. A blank chronicle until the world next
    // turns would be the exchange pretending history started when it opened.
    let view = world();
    let mut session = GameSession::new(&market(), &orders());
    assert!(session.chronicle().is_empty());

    session.seed_chronicle(&view.chronicle);
    let seeded = session.chronicle().len();

    assert_eq!(seeded, view.chronicle.len().min(CHRONICLE_CAPACITY));
    assert!(seeded > 0, "the seeded world writes a chronicle");

    // Seeding twice must not double the archive: after the first read the
    // lines arrive a year at a time.
    session.seed_chronicle(&view.chronicle);
    assert_eq!(session.chronicle().len(), seeded);
}

#[test]
fn a_turning_adds_its_prose_newest_first_and_the_run_stays_bounded() {
    let data = data();
    let mut session = GameSession::new(&market(), &orders());

    for year in 1..=200u32 {
        let mut turning = delta(year, year as u64);
        turning.events = vec![
            WorldEvent {
                year,
                kind: EventKind::Region,
                message: format!("the first thing that happened in {year}"),
            },
            WorldEvent {
                year,
                kind: EventKind::Hero,
                message: format!("and then the last thing in {year}"),
            },
        ];
        session.observe(Snapshot::default(), Some(&turning), 1, &data);
    }

    assert_eq!(session.chronicle().len(), CHRONICLE_CAPACITY);
    // Newest first, and a year's own lines keep the order the world wrote
    // them in — a chronicle read backwards within a year would be nonsense.
    assert_eq!(session.chronicle()[0].year, 200);
    assert!(session.chronicle()[0]
        .message
        .starts_with("the first thing"));
    assert!(session.chronicle()[1].message.starts_with("and then"));
}

#[test]
fn a_settlement_records_what_actually_moved_beside_the_prose() {
    // The honest cross-link (GDD 5.5): the prose and the price moves are
    // recorded side by side and nothing joins them but the year.
    let data = data();
    let view = world();
    let mut session = GameSession::new(&market(), &orders());
    session.observe(Snapshot::read(&view), None, 1, &data);

    // A year in which one province's fortunes collapse.
    let mut struck = view.clone();
    struck.regions[0].prosperity = 0.0;
    struck.regions[0].famine = true;
    let before = Snapshot::read(&view);
    let after = Snapshot::read(&struck);
    let mut turning = crate::world::delta::diff_snapshots(&before, &after);
    turning.to_year = view.year + 1;
    turning.events = vec![WorldEvent {
        year: view.year + 1,
        kind: EventKind::Region,
        message: "a grand and mythic sentence about something else".to_owned(),
    }];
    session.observe(after, Some(&turning), 1, &data);

    let record = &session.settlements()[0];
    assert!(!record.movers.is_empty(), "the floor moved on the news");
    assert!(
        record.movers[0].change.abs() >= record.movers[record.movers.len() - 1].change.abs(),
        "largest move first"
    );
    // Every mover names paper that is actually on the board, so the ticker
    // can be clicked through to its page.
    for mover in &record.movers {
        assert!(session.board().get(&mover.entity).is_some());
    }
    // And the prose rode along untouched, having priced nothing.
    assert_eq!(session.chronicle().len(), 1);
}

#[test]
fn an_unreadable_save_starts_fresh_rather_than_failing() {
    // The shipped config with only the version bumped, so a field added to
    // it later cannot make this test lie about what migration does.
    let config = GameConfig {
        version: "0.2.0".to_owned(),
        ..data().config
    };
    let value = serde_json::json!({ "nonsense": true });

    let migrated = migrate_save_value(Some("0.0.1".to_owned()), value, &config).unwrap();

    assert_eq!(migrated.version, "0.2.0");
    assert_eq!(migrated.cursor, 0);
    assert_eq!(migrated.last_tick, 0);
}

#[test]
fn a_current_save_survives_migration_with_its_cursor_intact() {
    // The shipped config with only the version bumped, so a field added to
    // it later cannot make this test lie about what migration does.
    let config = GameConfig {
        version: "0.2.0".to_owned(),
        ..data().config
    };
    let value = serde_json::json!({
        "version": "0.1.0",
        "cursor": 918,
        "last_tick": 16_290
    });

    let migrated = migrate_save_value(Some("0.1.0".to_owned()), value, &config).unwrap();

    assert_eq!(migrated.version, "0.2.0", "stamped to the current version");
    assert_eq!(migrated.cursor, 918, "the cursor is not lost");
    assert_eq!(migrated.last_tick, 16_290);
}

#[test]
fn an_age_ending_is_noticed_once_and_marks_the_settlement() {
    // The largest event in the game and the one nothing could have priced
    // (GDD 5.6). The exchange cannot see it coming — only notice it has
    // happened, and it must never notice the same one twice.
    let data = data();
    let view = world();
    let mut session = GameSession::new(&market(), &orders());

    // The first reading establishes the age; it is not itself a turning of one.
    let opening = session.observe(Snapshot::read(&view), None, 4, &data);
    assert!(!opening.cataclysm, "the first age read is not a new age");

    let quiet = delta(view.year + 1, 2);
    let same = session.observe(Snapshot::read(&view), Some(&quiet), 4, &data);
    assert!(!same.cataclysm);
    assert!(!session.settlements()[0].cataclysm);

    let turning = delta(view.year + 2, 3);
    let ended = session.observe(Snapshot::read(&view), Some(&turning), 5, &data);
    assert!(ended.cataclysm, "the age turned");
    assert!(session.settlements()[0].cataclysm);

    // And the next year in the new age is an ordinary year again.
    let after = delta(view.year + 3, 4);
    assert!(
        !session
            .observe(Snapshot::read(&view), Some(&after), 5, &data)
            .cataclysm
    );
}

#[test]
fn the_chart_can_tell_which_ages_a_listing_lived_through() {
    // The era series is what lets the chart rule off where an age ended, and it
    // has to survive decimation with the price beside it.
    let data = data();
    let view = world();
    let mut session = GameSession::new(&market(), &orders());
    session.observe(Snapshot::read(&view), None, 7, &data);

    for year in 1..=40u32 {
        let era = if year < 20 { 7 } else { 8 };
        session.observe(
            Snapshot::read(&view),
            Some(&delta(view.year + year, year as u64)),
            era,
            &data,
        );
    }

    let listing = session.board().listings().next().expect("paper is listed");
    assert_eq!(listing.era_history.min(), Some(7.0));
    assert_eq!(listing.era_history.max(), Some(8.0));
    assert_eq!(
        listing.era_history.len(),
        listing.history.len(),
        "the era series must stay aligned with the price it rules off"
    );
}

#[test]
fn a_licence_is_bought_with_the_same_cash_that_buys_paper() {
    // Pillar 3's whole tension: every crown spent on seeing further is a crown
    // not invested.
    let data = data();
    let mut session = GameSession::new(&market(), &orders());
    let opening = session.portfolio().cash;

    let next = data.licences.next(0).expect("the ladder has a second rung");
    let cost = next.cost;
    let name = session.buy_licence(&data.licences).expect("affordable");

    assert_eq!(name, next.name);
    assert_eq!(session.licences().held, 1);
    assert_eq!(session.portfolio().cash, opening - cost);
    // And the board widens: a class that was beyond reach now is not.
    assert!(session.licensed_for("regional_bond", &data.licences));
    assert!(!session.licensed_for("house_scrip", &data.licences));
}

#[test]
fn a_licence_beyond_the_ledger_is_refused_without_spending_anything() {
    let data = data();
    let mut session = GameSession::new(&market(), &orders());
    session.buy_licence(&data.licences).unwrap();
    // Drain the ledger, then try to climb again.
    let cash = session.portfolio().cash;
    session.portfolio.cash = 1.0;

    let refused = session.buy_licence(&data.licences).unwrap_err();

    assert!(
        matches!(refused, LicenceError::Insufficient { .. }),
        "{refused:?}"
    );
    assert_eq!(session.licences().held, 1, "and the rung was not climbed");
    assert_eq!(session.portfolio().cash, 1.0);
    let _ = cash;
}

#[test]
fn licences_survive_a_save() {
    let data = data();
    let mut session = GameSession::new(&market(), &orders());
    // Two rungs cost more than a broker opens with, deliberately — buying sight
    // is meant to hurt — so this fixture states its own wealth.
    session.portfolio.cash = 100_000.0;
    session.buy_licence(&data.licences).unwrap();
    session.buy_licence(&data.licences).unwrap();

    let resumed = GameSession::from_save(&session.to_save("0.1.0", 0), &market(), &orders());

    assert_eq!(resumed.licences().held, 2);
    assert!(resumed.licensed_for("house_scrip", &data.licences));
}
