//! The five-term model, term by term (GDD 5.4).

use super::*;
use crate::data::GameData;

fn config() -> MarketConfig {
    GameData::load()
        .expect("the exchange's data loads")
        .config
        .market
}

/// A market with the noise turned off, so a test can watch one term at a time.
fn silent() -> MarketConfig {
    MarketConfig {
        noise_scale: 0.0,
        ..config()
    }
}

fn rng() -> SeededRng {
    SeededRng::new(7)
}

#[test]
fn a_price_below_fair_value_drifts_up_toward_it_without_arriving() {
    let config = silent();
    let mut rng = rng();

    let moved = settle(80.0, 100.0, 0.1, 0.0, 0.0, &config, &mut rng);

    assert!(moved.to > 80.0 && moved.to < 100.0, "{moved:?}");
    assert_eq!(moved.drift, 20.0 * config.mean_reversion);
    // The gap is what a broker trades. If drift closed it in one settlement
    // there would be nothing to be early about.
    assert!(moved.to < 95.0, "the gap must survive the year: {moved:?}");
}

#[test]
fn the_terms_always_sum_to_the_move_that_happened() {
    // Pillar 1: a player must be able to ask why a price moved and get a
    // decomposition. One whose parts do not add up is worse than none.
    let config = config();
    let mut rng = rng();

    for (price, fair, shock, flow) in [
        (100.0, 130.0, 0.05, 0.0),
        (12.0, 4.0, -0.30, 0.02),
        (900.0, 900.0, 0.0, -0.01),
        (3.0, 60.0, 0.40, 0.10),
    ] {
        let moved = settle(price, fair, 0.15, shock, flow, &config, &mut rng);
        let summed = moved.drift + moved.shock + moved.noise + moved.flow;
        assert!(
            (summed - moved.change()).abs() < 0.001,
            "{summed} != {} in {moved:?}",
            moved.change()
        );
    }
}

#[test]
fn a_cataclysm_hurts_without_teleporting_and_the_parts_still_add_up() {
    let config = silent();
    let mut rng = rng();

    // A shock far larger than the cap allows.
    let moved = settle(100.0, 100.0, 0.1, -5.0, 0.0, &config, &mut rng);

    assert!(
        (moved.change() + 100.0 * config.max_move).abs() < 0.001,
        "the move is capped: {moved:?}"
    );
    // Capping truncates the move but must not truncate the explanation — the
    // shock term is rescaled, not clipped away.
    assert!((moved.shock - moved.change()).abs() < 0.001, "{moved:?}");
    assert_eq!(moved.drift, 0.0, "there was no gap to close");
}

#[test]
fn a_price_never_falls_through_the_floor() {
    let config = silent();
    let mut rng = rng();

    let mut price = 10.0;
    // Fair value at nothing, hit repeatedly: a claim on something worthless
    // still trades somewhere, and a price of zero would make every
    // fraction-of-price term below it dead arithmetic forever.
    for _ in 0..200 {
        price = settle(price, 0.0, 0.2, -1.0, 0.0, &config, &mut rng).to;
    }
    assert_eq!(price, config.min_price);

    // And the decomposition survives the floor too.
    let moved = settle(config.min_price, 0.0, 0.2, -1.0, 0.0, &config, &mut rng);
    let summed = moved.drift + moved.shock + moved.noise + moved.flow;
    assert!((summed - moved.change()).abs() < 0.001, "{moved:?}");
}

#[test]
fn the_same_seed_replays_the_same_century() {
    // GDD 14's determinism test at the level of one price: same seed, same
    // inputs, same history. Everything above this inherits it.
    let config = config();
    let run = |seed: u64| {
        let mut rng = SeededRng::new(seed);
        let mut price = 50.0;
        let mut path = Vec::new();
        for year in 0..100 {
            let fair = 50.0 + (year % 17) as f32;
            price = settle(price, fair, 0.12, 0.0, 0.0, &config, &mut rng).to;
            path.push(price);
        }
        path
    };

    assert_eq!(run(config.seed), run(config.seed));
    assert_ne!(
        run(config.seed),
        run(config.seed + 1),
        "a different seed must be a different century"
    );
}

#[test]
fn noise_alone_wanders_without_running_away() {
    // The noise term is bounded on purpose: the cap exists for cataclysms, not
    // to swallow arithmetic. A century of pure noise around a fixed fair value
    // must stay in a sane band.
    let config = config();
    let mut rng = SeededRng::new(99);
    let mut price = 100.0;
    let (mut low, mut high) = (price, price);

    for _ in 0..100 {
        price = settle(price, 100.0, 0.12, 0.0, 0.0, &config, &mut rng).to;
        low = low.min(price);
        high = high.max(price);
    }

    assert!(low > 40.0 && high < 250.0, "{low} .. {high}");
    // And mean reversion keeps pulling it home rather than letting it walk off.
    assert!((price - 100.0).abs() < 60.0, "ended at {price}");
}

#[test]
fn the_decomposition_leads_with_what_mattered_and_omits_what_did_not() {
    let moved = PriceMove {
        from: 100.0,
        drift: 1.0,
        shock: -8.0,
        noise: 0.5,
        flow: 0.0,
        to: 93.5,
    };

    let parts = moved.parts();
    assert_eq!(parts[0].0, "shock", "the biggest term leads");
    assert_eq!(parts.len(), 3, "a term that did nothing is not listed");
    assert!(parts.iter().all(|(name, _)| *name != "flow"));
    assert!((moved.change_fraction() + 0.065).abs() < 0.0001);
}

#[test]
fn a_new_listing_has_not_moved_yet() {
    let opened = PriceMove::unmoved(42.0);
    assert_eq!(opened.change(), 0.0);
    assert_eq!(opened.change_fraction(), 0.0);
    assert!(opened.parts().is_empty());
}
