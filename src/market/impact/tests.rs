//! Shocks under a real world (GDD 14's delta-coverage test).
//!
//! Fixtures are a real seeded Mytherra world projected through the real
//! spectator Standing, so a rule keyed on a field the world stopped carrying
//! fails here rather than going quietly dead in play.

use super::*;
use crate::data::GameData;
use crate::world::delta::diff;
use mytherra_core::data::ResourceStatus;
use mytherra_core::world::{RegionStatus, WorldState};
use mytherra_protocol::{project_world, spectator_standing, WorldView};

fn base() -> WorldView {
    let data = mytherra_core::data::GameData::load().expect("mytherra content loads");
    project_world(&WorldState::new(&data), &spectator_standing())
}

fn next(view: &WorldView) -> WorldView {
    let mut after = view.clone();
    after.tick_count += 1;
    after.year += 1;
    after
}

fn table() -> ImpactTable {
    GameData::load().expect("the exchange's data loads").impacts
}

/// Shock everything a turning of the world produced.
fn shocks_between(before: &WorldView, after: &WorldView) -> BTreeMap<EntityRef, f32> {
    table().shocks(&diff(before, after), &Snapshot::read(after))
}

#[test]
fn the_shipped_table_is_valid() {
    table().validate().expect("the shipped impacts are valid");
}

#[test]
fn a_rule_on_a_field_the_world_lacks_is_a_load_error() {
    let mut table = table();
    table.rules.push(ImpactRule {
        field: "the_mood_of_the_gods".to_owned(),
        class: "regional_bond".to_owned(),
        via: Via::SelfEntity,
        per_unit: 1.0,
        cap: 0.1,
        when: None,
        shock: 0.0,
    });

    let err = table.validate().unwrap_err();
    assert!(err.contains("the_mood_of_the_gods"), "{err}");
}

#[test]
fn a_rule_that_could_never_fire_is_a_load_error() {
    // A mistyped balance line looks exactly like a value someone deliberately
    // set to zero, so it has to be refused rather than tolerated.
    let mut table = table();
    table.rules.push(ImpactRule {
        field: "famine".to_owned(),
        class: "regional_bond".to_owned(),
        via: Via::SelfEntity,
        per_unit: 0.0,
        cap: 0.1,
        when: None,
        shock: -0.5,
    });

    let err = table.validate().unwrap_err();
    assert!(err.contains("can never fire"), "{err}");
}

#[test]
fn a_quiet_year_shocks_nothing() {
    let before = base();
    let after = next(&before);

    assert!(shocks_between(&before, &after).is_empty());
}

#[test]
fn a_famine_hits_the_province_and_the_towns_inside_it() {
    // The whole reason a shock travels: a famine is a fact about a region, and
    // the towns in it are what actually starve.
    let before = base();
    let mut after = next(&before);
    after.regions[0].famine = true;
    let region = EntityRef::Region(after.regions[0].id.clone());
    let towns: Vec<EntityRef> = before
        .settlements
        .iter()
        .filter(|s| s.region_id == after.regions[0].id)
        .map(|s| EntityRef::Settlement(s.id.clone()))
        .collect();
    assert!(!towns.is_empty(), "the fixture region holds towns");

    let shocks = shocks_between(&before, &after);

    assert!(shocks[&region] < 0.0, "the province is marked down");
    for town in &towns {
        assert!(shocks[town] < 0.0, "a town in a famine: {town:?}");
    }
    // Towns elsewhere are untouched — the shock travelled along the world's own
    // parentage, not across the whole board.
    let untouched = before
        .settlements
        .iter()
        .find(|s| s.region_id != after.regions[0].id)
        .expect("another region has towns");
    assert!(!shocks.contains_key(&EntityRef::Settlement(untouched.id.clone())));
}

#[test]
fn a_node_worked_out_makes_its_commodity_dearer() {
    // A node is not a listing, so this is the only route its news has to the
    // board — and scarcity means the shock is *upward*.
    let before = base();
    let mut after = next(&before);
    let fresh = before
        .resource_nodes
        .iter()
        .position(|n| n.status != ResourceStatus::Depleted)
        .expect("a producing node exists");
    let kind = after.resource_nodes[fresh].resource_type.label();
    after.resource_nodes[fresh].status = ResourceStatus::Depleted;

    let shocks = shocks_between(&before, &after);
    let commodity = EntityRef::Commodity(kind.to_owned());

    assert!(
        shocks[&commodity] > 0.0,
        "losing supply must make it dearer: {:?}",
        shocks.get(&commodity)
    );
    // The node itself is not listed, so no shock is recorded against it.
    assert!(!shocks.contains_key(&EntityRef::ResourceNode(
        after.resource_nodes[fresh].id.clone()
    )));
}

#[test]
fn a_scalar_shock_scales_with_how_far_the_field_went() {
    let before = base();
    let region = EntityRef::Region(before.regions[0].id.clone());

    let mut small = next(&before);
    small.regions[0].prosperity -= 5.0;
    let mut large = next(&before);
    large.regions[0].prosperity -= 20.0;

    let a = shocks_between(&before, &small)[&region];
    let b = shocks_between(&before, &large)[&region];

    assert!(a < 0.0 && b < a, "a bigger fall must hurt more: {a} vs {b}");
}

#[test]
fn no_single_year_may_shock_a_listing_past_the_total_cap() {
    // Everything going wrong at once is a cataclysm, not an infinity.
    let before = base();
    let mut after = next(&before);
    let struck = &mut after.regions[0];
    struck.prosperity = 0.0;
    struck.harvest = 0.0;
    struck.chaos = 100.0;
    struck.danger = 100.0;
    struck.strife = 100.0;
    struck.famine = true;
    struck.status = RegionStatus::WarTorn;
    let region = EntityRef::Region(struck.id.clone());

    let shock = shocks_between(&before, &after)[&region];

    assert!(shock < -0.3, "it should still be ruinous: {shock}");
    assert!(shock >= -table().total_cap, "but bounded: {shock}");
}

#[test]
fn the_chronicle_is_never_read_by_the_pricer() {
    // Pillar 4 made mechanical: the prose is the newspaper, the deltas are the
    // truth. A turning that carries only prose must shock nothing at all.
    let before = base();
    let after = next(&before);
    let mut delta = diff(&before, &after);
    delta.events = before.chronicle.clone();
    assert!(!delta.events.is_empty(), "the fixture carries prose");

    assert!(table().shocks(&delta, &Snapshot::read(&after)).is_empty());
}
