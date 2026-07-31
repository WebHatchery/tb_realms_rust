//! Execution, and the two ways size costs you (GDD open question 3).

use super::*;
use crate::data::GameData;
use crate::market::pricing::PriceMove;
use macroquad_toolkit::series::Series;

fn config() -> OrderConfig {
    GameData::load()
        .expect("the exchange's data loads")
        .config
        .orders
}

/// A listing at a chosen price and depth, with no world behind it — this file
/// is about arithmetic, and the world's part is tested elsewhere.
fn listing(price: f32, liquidity: f32) -> Instrument {
    let mut history = Series::new(64);
    history.push(price);
    let mut fair_history = Series::new(64);
    fair_history.push(price);
    Instrument {
        symbol: "TEST".to_owned(),
        name: "A Test Claim".to_owned(),
        entity: EntityRef::Region("test".to_owned()),
        class: "Regional bond".to_owned(),
        fair_value: price,
        price,
        previous_close: price,
        income: 0.0,
        volatility: 0.1,
        liquidity,
        history,
        fair_history,
        last_move: PriceMove::unmoved(price),
        listed_year: 1,
    }
}

#[test]
fn slippage_is_always_against_the_trader() {
    let config = config();
    let listing = listing(100.0, 0.5);

    let bought = quote(&listing, Side::Buy, 100, 1, &config);
    let sold = quote(&listing, Side::Sell, 100, 1, &config);

    assert!(bought.price > 100.0, "buying dear: {}", bought.price);
    assert!(sold.price < 100.0, "selling cheap: {}", sold.price);
    // Symmetric in magnitude — the floor is not picking a direction, it is
    // charging for the size.
    assert!((bought.slippage - sold.slippage).abs() < 0.0001);
}

#[test]
fn a_bigger_order_gets_a_worse_fill() {
    let config = config();
    let listing = listing(100.0, 0.5);

    let small = quote(&listing, Side::Buy, 10, 1, &config);
    let large = quote(&listing, Side::Buy, 200, 1, &config);

    assert!(large.price > small.price);
    assert!(
        large.flow.abs() > small.flow.abs(),
        "and leaves a bigger mark"
    );
}

#[test]
fn a_thin_listing_punishes_a_size_a_deep_one_absorbs() {
    // GDD 5.2 promises relic lots "enormous spreads". Liquidity is where that
    // promise is kept, and it is the same order in both cases.
    //
    // Five units, deliberately small: a size large enough to cap the relic
    // would compress the ratio and measure the cap instead of the depth.
    let config = config();
    let bond = quote(&listing(100.0, 0.85), Side::Buy, 5, 1, &config);
    let relic = quote(&listing(100.0, 0.06), Side::Buy, 5, 1, &config);

    assert!(relic.slippage < config.max_slippage, "not capped yet");
    assert!(
        relic.slippage > bond.slippage * 5.0,
        "bond {} vs relic {}",
        bond.slippage,
        relic.slippage
    );
}

#[test]
fn slippage_stops_getting_worse_past_the_cap() {
    // Past a point the honest answer is that the order is simply too big, and a
    // cap makes that visible as a fill that stops responding to size.
    let config = config();
    let listing = listing(100.0, 0.1);

    let huge = quote(&listing, Side::Buy, 100_000, 1, &config);
    let absurd = quote(&listing, Side::Buy, 10_000_000, 1, &config);

    assert_eq!(huge.slippage, config.max_slippage);
    assert_eq!(absurd.slippage, config.max_slippage);
}

#[test]
fn a_tiny_order_still_pays_the_minimum_fee() {
    // Otherwise a thousand one-unit trades would be a free way to do anything.
    let config = config();
    let fill = quote(&listing(1.0, 0.9), Side::Buy, 1, 1, &config);

    assert_eq!(fill.fee, config.minimum_fee);
    assert!(fill.fee > fill.gross * config.commission);
}

#[test]
fn an_order_of_nothing_costs_nothing() {
    let config = config();
    let fill = quote(&listing(100.0, 0.5), Side::Buy, 0, 1, &config);

    assert_eq!(fill.units, 0);
    assert_eq!(fill.fee, 0.0);
    assert_eq!(fill.cash, 0.0);
    assert_eq!(fill.flow, 0.0);
}

#[test]
fn buying_marks_the_price_up_and_selling_marks_it_down() {
    // The flow term is signed: your own order pushes the price the way you
    // traded, and you are the one who then has to trade back through it.
    let config = config();
    let listing = listing(100.0, 0.5);

    assert!(quote(&listing, Side::Buy, 50, 1, &config).flow > 0.0);
    assert!(quote(&listing, Side::Sell, 50, 1, &config).flow < 0.0);
}

#[test]
fn friction_is_what_the_size_and_the_fees_cost_above_the_screen_price() {
    let config = config();
    let listing = listing(100.0, 0.4);
    let fill = quote(&listing, Side::Buy, 80, 1, &config);

    let screen = 100.0 * 80.0;
    assert!((fill.friction() - ((fill.gross - screen) + fill.fee)).abs() < 0.01);
    assert!(
        fill.friction() > fill.fee,
        "at this size, slippage dwarfs commission"
    );
}
