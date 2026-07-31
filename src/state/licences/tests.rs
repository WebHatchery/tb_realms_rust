//! The information ladder (GDD 5.3), and the shape it has to keep.

use super::*;
use crate::data::GameData;
use crate::world::fundamentals::EntityRef;

fn table() -> LicenceTable {
    GameData::load()
        .expect("the exchange's data loads")
        .licences
}

#[test]
fn the_shipped_ladder_reaches_every_class_exactly_once() {
    // A class under no tier would be paper no licence could ever trade; a class
    // under two would make the ladder's order meaningless.
    let table = table();
    table.validate().expect("the shipped ladder is valid");

    let top = table.top();
    for class in EntityRef::LISTING_CLASSES {
        assert!(
            table.permits(top, class),
            "the top of the ladder cannot reach {class}"
        );
    }
}

#[test]
fn the_first_rung_is_free_and_reaches_only_the_coarsest_paper() {
    // A broker starts on the floor knowing the tide and not the boat.
    let table = table();
    assert_eq!(table.current(0).cost, 0.0);

    assert!(table.permits(0, "commodity"));
    assert!(!table.permits(0, "house_scrip"));
    assert!(!table.permits(0, "regional_bond"));
    assert!(!table.is_archivist(0), "and cannot read the archive");
}

#[test]
fn each_rung_keeps_everything_beneath_it() {
    // Cumulative, or a broker would trade away their own sight by climbing.
    let table = table();
    for rung in 0..=table.top() {
        for lower in 0..rung {
            for class in EntityRef::LISTING_CLASSES {
                if table.permits(lower, class) {
                    assert!(
                        table.permits(rung, class),
                        "rung {rung} lost {class}, which rung {lower} had"
                    );
                }
            }
        }
    }
}

#[test]
fn only_the_top_rung_opens_the_archive() {
    let table = table();
    for rung in 0..table.top() {
        assert!(!table.is_archivist(rung), "rung {rung} reads the archive");
    }
    assert!(table.is_archivist(table.top()));
}

#[test]
fn a_class_no_licence_reaches_is_a_load_error() {
    let mut table = table();
    let orphaned = table.tiers[0].classes.remove(0);

    let err = table.validate().unwrap_err();
    assert!(err.contains(&orphaned), "{err}");
    assert!(err.contains("no licence"), "{err}");
}

#[test]
fn a_class_two_licences_reach_is_a_load_error() {
    let mut table = table();
    let doubled = table.tiers[0].classes[0].clone();
    table.tiers[1].classes.push(doubled.clone());

    let err = table.validate().unwrap_err();
    assert!(err.contains(&doubled), "{err}");
}

#[test]
fn a_scope_mytherra_does_not_define_is_a_load_error() {
    // The tiers are designed against Mytherra's own visibility vocabulary, so a
    // typo there should fail loudly rather than name nothing.
    let mut table = table();
    table.tiers[1].scopes.push("the_minds_of_gods".to_owned());

    let err = table.validate().unwrap_err();
    assert!(err.contains("the_minds_of_gods"), "{err}");
}

#[test]
fn a_ladder_whose_first_rung_costs_money_is_a_load_error() {
    // A broker who cannot afford to start could never start.
    let mut table = table();
    table.tiers[0].cost = 10.0;

    assert!(table.validate().unwrap_err().contains("must be free"));
}

#[test]
fn the_top_of_the_ladder_has_no_next_rung() {
    let table = table();
    assert!(table.next(table.top()).is_none());
    // And asking about a rung past the top is answered, not panicked over.
    assert_eq!(table.current(999).id, table.current(table.top()).id);
}
