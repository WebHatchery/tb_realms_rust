//! The taxonomy under a real world (GDD 14).
//!
//! Fixtures come from a **real seeded Mytherra world** projected through the
//! real spectator Standing, not from hand-written struct literals — so a rule
//! that prices on a field the world stopped carrying fails here rather than in
//! play, and the no-orphan-instrument check is made against the world that
//! actually ships.

use super::*;
use crate::data::GameData;
use mytherra_core::data::ResourceStatus;
use mytherra_core::world::{House, WorldState};
use mytherra_protocol::{project_world, spectator_standing};

/// A freshly seeded world as a spectator receives it.
fn base() -> mytherra_protocol::WorldView {
    let data = mytherra_core::data::GameData::load().expect("mytherra content loads");
    let world = WorldState::new(&data);
    project_world(&world, &spectator_standing())
}

fn rules() -> InstrumentRules {
    GameData::load()
        .expect("the exchange's data loads")
        .instruments
}

fn market() -> MarketConfig {
    GameData::load()
        .expect("the exchange's data loads")
        .config
        .market
}

/// A board opened straight onto a world.
fn board_over(view: &mytherra_protocol::WorldView) -> (Board, InstrumentRules) {
    let rules = rules();
    let mut board = Board::new();
    board.refresh(&Snapshot::read(view), &rules, &market(), 1);
    (board, rules)
}

fn listing_named(board: &Board, name: &str) -> Instrument {
    board
        .listings()
        .find(|listing| listing.name == name)
        .unwrap_or_else(|| panic!("no listing named {name}"))
        .clone()
}

#[test]
fn the_embedded_taxonomy_covers_every_class_exactly_once() {
    // The no-orphan-instrument rule (GDD 14) at the data layer: a class with no
    // rule would be an entity nobody could ever trade.
    let rules = rules();
    rules.validate().expect("the shipped taxonomy is valid");
    assert_eq!(rules.classes.len(), EntityRef::LISTING_CLASSES.len());
    for class in EntityRef::LISTING_CLASSES {
        assert!(
            rules.classes.iter().any(|rule| rule.id == class),
            "no rule for {class}"
        );
    }
}

#[test]
fn a_rule_pricing_on_a_field_the_world_lacks_is_a_load_error() {
    let mut rules = rules();
    rules.classes[0].terms.push(Term {
        field: "the_mood_of_the_gods".to_owned(),
        weight: 1.0,
    });

    let err = rules.validate().unwrap_err();
    assert!(err.contains("the_mood_of_the_gods"), "{err}");
}

#[test]
fn a_fresh_world_lists_every_class_it_can_back() {
    let view = base();
    let (board, _) = board_over(&view);

    let classes: std::collections::BTreeSet<&str> = board
        .listings()
        .map(|listing| listing.class.as_str())
        .collect();

    // Houses and Orders are deliberately absent: both arise dynamically from
    // heroes, so a year-one world has none. Everything else must be there, or
    // the taxonomy names something the world cannot back.
    for class in [
        "Regional bond",
        "Settlement share",
        "Route toll",
        "Commodity",
        "Relic lot",
    ] {
        assert!(classes.contains(class), "nothing listed in {class}");
    }

    // One listing per backing entity, plus one per resource *kind* rather than
    // per node — the nodes are supply, the kind is the security.
    let kinds: std::collections::BTreeSet<&str> = view
        .resource_nodes
        .iter()
        .map(|node| node.resource_type.label())
        .collect();
    assert_eq!(
        board.len(),
        view.regions.len()
            + view.settlements.len()
            + view.trade_routes.len()
            + view.artifacts.len()
            + kinds.len()
    );
}

#[test]
fn a_resource_node_is_watched_but_never_listed() {
    // You trade grain, not a named field. The nodes are supply behind the
    // commodity, and the exchange must not offer paper on one.
    let view = base();
    let (board, _) = board_over(&view);

    for node in &view.resource_nodes {
        let entity = EntityRef::ResourceNode(node.id.clone());
        assert!(
            board.listings().all(|listing| listing.entity != entity),
            "a node was listed"
        );
    }
    assert!(board.listings().any(|l| l.class == "Commodity"));
}

#[test]
fn every_listing_carries_a_symbol_no_other_listing_holds() {
    let view = base();
    let (board, _) = board_over(&view);

    let symbols: std::collections::BTreeSet<&str> =
        board.listings().map(|l| l.symbol.as_str()).collect();
    assert_eq!(symbols.len(), board.len(), "two listings share a ticker");
    assert!(board.listings().all(|l| l.symbol.len() >= 4));
}

#[test]
fn a_famine_marks_a_regional_bond_down() {
    let view = base();
    let (mut board, rules) = board_over(&view);
    let region = view.regions[0].clone();
    let before = listing_named(&board, &region.name).fair_value;

    let mut struck = view.clone();
    struck.regions[0].famine = true;
    let change = board.refresh(&Snapshot::read(&struck), &rules, &market(), 1);
    let after = listing_named(&board, &region.name).fair_value;

    assert!(
        after < before,
        "a famine must mark the province down: {before} -> {after}"
    );
    assert!(change.revalued >= 1);
    assert!(change.listed.is_empty() && change.delisted.is_empty());
}

#[test]
fn a_supply_shock_makes_a_commodity_dearer_rather_than_cheaper() {
    // The scarcity rule is the whole reason commodities price differently from
    // everything else: a storm that ruins the fields does not make grain worth
    // less, it makes it worth more.
    let view = base();
    let (mut board, rules) = board_over(&view);
    let kind = view.resource_nodes[0].resource_type;
    let label = kind.label();
    let before = listing_named(&board, label).fair_value;

    let mut struck = view.clone();
    for node in &mut struck.resource_nodes {
        if node.resource_type == kind {
            node.status = ResourceStatus::Depleted;
        }
    }
    board.refresh(&Snapshot::read(&struck), &rules, &market(), 1);
    let after = listing_named(&board, label).fair_value;

    assert!(
        after > before * 2.0,
        "losing every producing node must spike {label}: {before} -> {after}"
    );
}

#[test]
fn a_house_lists_when_it_is_founded_and_delists_when_its_blood_runs_out() {
    // The content no authored market can produce (GDD 8).
    let view = base();
    let (mut board, rules) = board_over(&view);
    let opened = board.len();

    let mut founded = view.clone();
    founded.houses.push(House {
        id: "aldwin".to_owned(),
        name: "The House of Brogan Aldwin".to_owned(),
        seat_region_id: view.regions[0].id.clone(),
        founder_name: "Brogan Aldwin".to_owned(),
        member_ids: vec![founded.heroes[0].id.clone()],
        prestige: 400.0,
    });
    let listing = board.refresh(&Snapshot::read(&founded), &rules, &market(), 1);
    assert_eq!(listing.listed, vec!["HBAL".to_owned()]);
    assert_eq!(board.len(), opened + 1);

    // The line dies out and the paper is struck off — the exchange does not keep
    // trading a claim on something that no longer exists.
    let struck = board.refresh(&Snapshot::read(&view), &rules, &market(), 1);
    assert_eq!(struck.delisted, vec!["HBAL".to_owned()]);
    assert_eq!(board.len(), opened);

    // And the ticker is free again, so a later house may take it.
    let relisted = board.refresh(&Snapshot::read(&founded), &rules, &market(), 1);
    assert_eq!(relisted.listed, vec!["HBAL".to_owned()]);
}

#[test]
fn a_houses_worth_follows_its_living_renown_not_its_lineage() {
    let mut view = base();
    let legend = view.heroes[0].id.clone();
    let cousin = view.heroes[1].id.clone();
    view.heroes[0].is_alive = true;
    view.heroes[0].renown = 900.0;
    view.heroes[1].is_alive = true;
    view.heroes[1].renown = 100.0;
    view.houses = vec![House {
        id: "aldwin".to_owned(),
        name: "Aldwin".to_owned(),
        seat_region_id: view.regions[0].id.clone(),
        founder_name: "Brogan".to_owned(),
        member_ids: vec![legend, cousin],
        prestige: 500.0,
    }];
    let (mut board, rules) = board_over(&view);
    let before = listing_named(&board, "Aldwin").fair_value;

    // The legend dies. `member_ids` keeps the dead, so the lineage is unchanged
    // and only the living figures move — nine tenths of the renown is gone.
    let mut bereft = view.clone();
    bereft.heroes[0].is_alive = false;
    board.refresh(&Snapshot::read(&bereft), &rules, &market(), 1);
    let after = listing_named(&board, "Aldwin").fair_value;

    assert!(
        after < before,
        "the house must be marked down: {before} -> {after}"
    );
    // Prestige has not moved yet — it only drifts toward renown over following
    // years — so the whole fall here is the renown and headcount terms. That
    // lag is precisely the gap a broker trades.
    let expected = 0.04 * 900.0 + 4.0;
    assert!((before - after - expected).abs() < 0.01, "{before} {after}");
}

#[test]
fn yield_is_real_income_rather_than_a_declared_ratio() {
    let view = base();
    let (board, _) = board_over(&view);

    let bond = board
        .listings()
        .find(|l| l.class == "Regional bond")
        .expect("a province is listed");
    assert!(bond.income > 0.0, "a bond pays a coupon from prosperity");
    assert!(
        bond.yield_rate() > 0.0 && bond.yield_rate() < 0.25,
        "a plausible coupon, not a decorative number: {}",
        bond.yield_rate()
    );

    // A relic pays nothing — it is a collectible, and all of its return is the
    // price.
    let relic = board
        .listings()
        .find(|l| l.class == "Relic lot")
        .expect("a relic is listed");
    assert_eq!(relic.income, 0.0);
    assert_eq!(relic.yield_rate(), 0.0);
}

#[test]
fn a_ruined_province_is_floored_rather_than_priced_below_nothing() {
    let view = base();
    let (mut board, rules) = board_over(&view);
    let region = view.regions[0].clone();

    let mut ruined = view.clone();
    let struck = &mut ruined.regions[0];
    struck.prosperity = 0.0;
    struck.harvest = 0.0;
    struck.cultural_influence = 0.0;
    struck.lore = 0.0;
    struck.chaos = 100.0;
    struck.danger = 100.0;
    struck.strife = 100.0;
    struck.famine = true;
    board.refresh(&Snapshot::read(&ruined), &rules, &market(), 1);

    let value = listing_named(&board, &region.name).fair_value;
    assert!(value > 0.0, "a claim never prices through zero: {value}");
    // The floor is applied before the crisis multipliers, so a ruined province
    // still trades below a merely poor one — the floor stops the arithmetic
    // going negative, it does not put a price under the disaster.
    assert!(value < 5.0, "and it is nearly worthless: {value}");
}

#[test]
fn tickers_stand_for_the_name_and_skip_the_kind_of_thing_it_is() {
    assert_eq!(ticker_core("Aldermoor"), "ALD");
    assert_eq!(ticker_core("The Iron Road"), "IRO");
    assert_eq!(ticker_core("The House of Brogan Aldwin"), "BAL");
    assert_eq!(ticker_core("the Arcane Circle"), "ACI");
    assert_eq!(ticker_core("Farmland"), "FAR");
    assert_eq!(ticker_core("Aegis of Aldermoor"), "AAL");
    // Nothing usable still yields something the board can print.
    assert_eq!(ticker_core("的"), "XXX");
}

#[test]
fn two_listings_with_the_same_letters_are_told_apart() {
    let view = base();
    let (mut board, rules) = board_over(&view);

    let mut twins = view.clone();
    for id in ["one", "two"] {
        twins.houses.push(House {
            id: id.to_owned(),
            name: "House Grey".to_owned(),
            seat_region_id: view.regions[0].id.clone(),
            founder_name: "Grey".to_owned(),
            member_ids: Vec::new(),
            prestige: 10.0,
        });
    }
    let change = board.refresh(&Snapshot::read(&twins), &rules, &market(), 1);

    assert_eq!(change.listed, vec!["HGRE".to_owned(), "HGRE2".to_owned()]);
}
