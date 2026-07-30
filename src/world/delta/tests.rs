//! Delta coverage (GDD 14): every field the exchange prices on must be
//! exercised by at least one fixture, so this fails loudly if Mytherra's model
//! changes underneath it — the main risk of the tie-in.
//!
//! Fixtures are built from a **real seeded world** projected through the real
//! spectator Standing, not from hand-written struct literals. That costs a
//! little setup and buys a lot: the tests exercise the same `WorldView` the
//! server actually sends, and a field added to Mytherra's model can never
//! silently bypass them.

use super::*;
use mytherra_core::data::{GameData, ResourceStatus};
use mytherra_core::world::{House, Order, RegionStatus, WorldState};
use mytherra_protocol::{project_world, spectator_standing};

/// A freshly seeded world as a spectator receives it.
fn base() -> WorldView {
    let data = GameData::load().expect("mytherra content loads");
    let world = WorldState::new(&data);
    project_world(&world, &spectator_standing())
}

/// `base()` advanced by one tick and year, with nothing else changed — the
/// starting point for "and then exactly this happened".
fn next(view: &WorldView) -> WorldView {
    let mut after = view.clone();
    after.tick_count += 1;
    after.year += 1;
    after
}

fn house(id: &str, prestige: f32, members: &[&str]) -> House {
    House {
        id: id.to_owned(),
        name: id.to_owned(),
        seat_region_id: "aldermoor".to_owned(),
        founder_name: "Founder".to_owned(),
        member_ids: members.iter().map(|m| (*m).to_owned()).collect(),
        prestige,
    }
}

/// Find the single move for a field, failing readably if the diff produced none.
fn move_for(delta: &WorldDelta, field: Field) -> &FieldMove {
    delta
        .moves
        .iter()
        .find(|m| m.field == field)
        .unwrap_or_else(|| panic!("no move recorded for {}", field.key()))
}

#[test]
fn the_seeded_world_supplies_every_instrument_class_we_diff() {
    // The no-orphan-instrument test (GDD 14): a class with no backing entity in
    // a fresh world would be an instrument nobody could ever trade. Houses and
    // Orders are excluded on purpose — both arise dynamically from heroes, so a
    // year-one world has none, which is itself a fact the exchange must handle.
    let view = base();
    assert!(!view.regions.is_empty(), "regional bonds");
    assert!(!view.settlements.is_empty(), "settlement shares");
    assert!(!view.trade_routes.is_empty(), "route tolls");
    assert!(!view.resource_nodes.is_empty(), "commodities");
    assert!(!view.artifacts.is_empty(), "relic lots");
    assert!(
        !view.heroes.is_empty(),
        "houses and orders derive from heroes"
    );
}

#[test]
fn a_famine_is_reported_as_a_harvest_fall_and_a_flag_flip() {
    let before = base();
    let mut after = next(&before);
    let struck = &mut after.regions[0];
    let region_id = struck.id.clone();
    let harvest_before = struck.harvest;
    struck.harvest -= 30.0;
    struck.famine = true;

    let delta = diff(&before, &after);

    assert_eq!(delta.years(), 1);
    assert_eq!(
        move_for(&delta, Field::Harvest).change,
        Change::Scalar {
            before: harvest_before,
            after: harvest_before - 30.0
        }
    );
    assert_eq!(move_for(&delta, Field::Harvest).change.magnitude(), -30.0);
    assert_eq!(
        move_for(&delta, Field::Famine).change,
        Change::Flag {
            before: false,
            after: true
        }
    );
    assert_eq!(
        move_for(&delta, Field::Harvest).entity,
        EntityRef::Region(region_id)
    );
}

#[test]
fn an_unmoved_world_produces_no_moves() {
    let before = base();
    let after = next(&before);

    let delta = diff(&before, &after);

    assert!(delta.is_empty(), "a quiet year moves nothing: {delta:?}");
    assert_eq!(delta.to_tick, before.tick_count + 1);
}

#[test]
fn float_noise_below_the_epsilon_is_not_a_move() {
    let before = base();
    let mut after = next(&before);
    // Smaller than SCALAR_EPSILON: the wobble an f32 sum produces without
    // anything in the world actually having happened.
    after.regions[0].prosperity += 0.000_02;
    after.trade_routes[0].volume += 0.000_05;

    assert!(diff(&before, &after).is_empty());
}

#[test]
fn a_region_status_change_is_reported_by_label() {
    let before = base();
    let mut after = next(&before);
    let was = after.regions[0].status;
    // Pick any status the region is not already in.
    let now = if was == RegionStatus::WarTorn {
        RegionStatus::Thriving
    } else {
        RegionStatus::WarTorn
    };
    after.regions[0].status = now;

    let delta = diff(&before, &after);

    assert_eq!(
        move_for(&delta, Field::RegionStatus).change,
        Change::Status {
            before: format!("{was:?}"),
            after: format!("{now:?}")
        }
    );
}

#[test]
fn a_house_losing_its_legend_shows_renown_falling_faster_than_its_numbers() {
    let mut before = base();
    // Two living members of one house, one of them carrying most of the renown.
    let legend = before.heroes[0].id.clone();
    let cousin = before.heroes[1].id.clone();
    before.heroes[0].is_alive = true;
    before.heroes[0].renown = 900.0;
    before.heroes[1].is_alive = true;
    before.heroes[1].renown = 100.0;
    before.houses = vec![house("aldwin", 500.0, &[&legend, &cousin])];

    // The legend dies. The lineage is unchanged — `member_ids` keeps the dead —
    // so only the living figures move.
    let mut after = next(&before);
    after.heroes[0].is_alive = false;
    after.houses[0].prestige = 480.0;

    let delta = diff(&before, &after);

    assert_eq!(
        move_for(&delta, Field::LivingMembers).change,
        Change::Scalar {
            before: 2.0,
            after: 1.0
        }
    );
    // The point of tracking both: one of two members died, but nine tenths of
    // the house's renown went with him. A pricer reading only the headcount
    // would misprice this badly.
    assert_eq!(
        move_for(&delta, Field::LivingRenown).change,
        Change::Scalar {
            before: 1000.0,
            after: 100.0
        }
    );
    // Prestige drifts toward that renown rather than jumping to it, so it lags —
    // and that gap is what a broker trades.
    assert_eq!(move_for(&delta, Field::Prestige).change.magnitude(), -20.0);
}

#[test]
fn an_orders_fellowship_is_counted_by_role_across_the_world() {
    let mut before = base();
    // Silence the seeded roster so the fixture states its own membership.
    for hero in &mut before.heroes {
        hero.is_alive = false;
    }
    let role = before.heroes[0].role;
    let other_role = before
        .heroes
        .iter()
        .map(|h| h.role)
        .find(|r| *r != role)
        .expect("the seeded world holds more than one calling");

    before.heroes[0].is_alive = true;
    before.heroes[0].renown = 50.0;
    before.heroes[0].role = role;
    before.heroes[1].is_alive = true;
    before.heroes[1].renown = 30.0;
    before.heroes[1].role = role;
    before.heroes[2].is_alive = true;
    before.heroes[2].renown = 400.0;
    before.heroes[2].role = other_role;

    before.orders = vec![Order {
        id: "circle".to_owned(),
        name: "the Arcane Circle".to_owned(),
        role,
        prestige: 60.0,
        founded_year: before.year,
    }];

    let mut after = next(&before);
    // One of the fellowship dies; the outsider's fortunes soar and must not count.
    after.heroes[1].is_alive = false;
    after.heroes[2].renown = 900.0;

    let delta = diff(&before, &after);

    assert_eq!(
        move_for(&delta, Field::LivingMembers).change,
        Change::Scalar {
            before: 2.0,
            after: 1.0
        }
    );
    assert_eq!(
        move_for(&delta, Field::LivingRenown).change,
        Change::Scalar {
            before: 80.0,
            after: 50.0
        },
        "an outsider's renown must not leak into another calling's order"
    );
}

#[test]
fn entities_appearing_and_vanishing_are_listed_and_delisted() {
    let mut before = base();
    before.houses = vec![house("old-line", 200.0, &[])];
    let razed = before.settlements[0].id.clone();

    let mut after = next(&before);
    // The old line dies out, a new one is founded, and a settlement is razed.
    after.houses = vec![house("new-line", 10.0, &[])];
    after.settlements.remove(0);

    let delta = diff(&before, &after);

    assert_eq!(delta.listed, vec![EntityRef::House("new-line".to_owned())]);
    assert_eq!(
        delta.delisted,
        vec![
            EntityRef::House("old-line".to_owned()),
            EntityRef::Settlement(razed),
        ]
    );
    // A delisted entity contributes no field moves — there is nothing left to
    // price, only a position to close.
    assert!(delta.moves.is_empty(), "{:?}", delta.moves);
}

#[test]
fn a_resource_node_status_change_is_a_supply_shock() {
    let before = base();
    let mut after = next(&before);
    let was = after.resource_nodes[0].status;
    let now = if was == ResourceStatus::Contested {
        ResourceStatus::Depleted
    } else {
        ResourceStatus::Contested
    };
    let node_id = after.resource_nodes[0].id.clone();
    after.resource_nodes[0].status = now;

    let delta = diff(&before, &after);

    assert_eq!(
        move_for(&delta, Field::NodeStatus).change,
        Change::Status {
            before: format!("{was:?}"),
            after: format!("{now:?}")
        }
    );
    let entity = EntityRef::ResourceNode(node_id);
    assert_eq!(move_for(&delta, Field::NodeStatus).entity, entity);
    // The node feeds a commodity but is never itself a listing — you trade
    // grain, not a named field.
    assert!(entity.listing_class().is_none());
    let kind = before.resource_nodes[0].resource_type.label();
    assert_eq!(
        EntityRef::Commodity(kind.to_owned()).listing_class(),
        Some("commodity")
    );
}

#[test]
fn the_last_node_of_a_kind_going_delists_its_commodity() {
    let before = base();
    let kind = before.resource_nodes[0].resource_type;
    let label = kind.label();
    let mut after = next(&before);
    // Every node of one kind is lost — the commodity has nothing left behind it.
    after.resource_nodes.retain(|n| n.resource_type != kind);

    let delta = diff(&before, &after);

    assert!(
        delta
            .delisted
            .contains(&EntityRef::Commodity(label.to_owned())),
        "a commodity with no nodes left stops being a listing: {:?}",
        delta.delisted
    );
}

#[test]
fn a_worked_out_mine_shows_up_as_supply_leaving_the_commodity() {
    let before = base();
    let kind = before.resource_nodes[0].resource_type;
    let mut after = next(&before);
    // One node of the kind is worked out. The commodity keeps its node count and
    // loses one producer — which is the number a supply shock is priced against.
    let already_dead = before.resource_nodes[0].status == ResourceStatus::Depleted;
    after.resource_nodes[0].status = ResourceStatus::Depleted;

    let delta = diff(&before, &after);
    let commodity = EntityRef::Commodity(kind.label().to_owned());
    let producing = delta
        .moves
        .iter()
        .find(|m| m.entity == commodity && m.field == Field::NodesProducing);

    if already_dead {
        assert!(producing.is_none(), "nothing changed, so nothing moved");
    } else {
        assert_eq!(producing.expect("supply moved").change.magnitude(), -1.0);
        assert!(
            !delta
                .moves
                .iter()
                .any(|m| m.entity == commodity && m.field == Field::NodeCount),
            "the node still exists; only its yield is gone"
        );
    }
}

#[test]
fn a_delta_can_span_more_than_one_year() {
    let before = base();
    let mut after = before.clone();
    after.tick_count += 10;
    after.year += 10;
    after.regions[0].prosperity += 30.0;

    let delta = diff(&before, &after);

    assert_eq!(delta.years(), 10, "a decade away from the wire still diffs");
    assert_eq!(move_for(&delta, Field::Prosperity).change.magnitude(), 30.0);
}

#[test]
fn the_digest_leads_with_the_largest_moves() {
    let before = base();
    let mut after = next(&before);
    after.regions[0].prosperity += 4.0;
    after.regions[0].chaos += 30.0;
    after.regions[1].prosperity -= 12.0;
    // A flag carries no magnitude, so it sorts last — but is never dropped.
    after.regions[1].famine = true;

    let delta = diff(&before, &after);
    let ranked = delta.most_moved(4);

    assert_eq!(ranked.len(), 4);
    assert_eq!(ranked[0].field, Field::Chaos, "the biggest move leads");
    assert_eq!(ranked[0].change.magnitude(), 30.0);
    assert_eq!(ranked[1].field, Field::Prosperity);
    assert_eq!(ranked[1].change.magnitude(), -12.0);
    assert_eq!(
        ranked[3].field,
        Field::Famine,
        "a magnitude-less change sorts last rather than vanishing"
    );

    // The limit truncates rather than failing when there is more to say.
    assert_eq!(delta.most_moved(2).len(), 2);
}

#[test]
fn every_watched_field_has_a_distinct_data_key_that_round_trips() {
    // `instruments.json` and `impacts.json` key on these strings (GDD 6), so a
    // duplicate would make two different world fields indistinguishable to the
    // balance tables, and a key that cannot be parsed back would make a rule
    // referencing it unreachable.
    let keys: std::collections::BTreeSet<&str> =
        Field::ALL.iter().map(|field| field.key()).collect();
    assert_eq!(keys.len(), Field::ALL.len(), "two fields share a data key");
    for field in Field::ALL {
        assert_eq!(Field::from_key(field.key()), Some(field));
    }
    assert_eq!(Field::from_key("not_a_field"), None);
}
