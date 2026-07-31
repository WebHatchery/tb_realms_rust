//! The ledger: cost basis, P&L, and the two rules that give it its shape.

use super::*;
use crate::data::GameData;
use crate::market::pricing::MarketConfig;
use crate::world::Snapshot;
use mytherra_protocol::{project_world, spectator_standing, WorldView};

fn data() -> GameData {
    GameData::load().expect("the exchange's data loads")
}

fn world() -> WorldView {
    let content = mytherra_core::data::GameData::load().expect("mytherra content loads");
    project_world(
        &mytherra_core::world::WorldState::new(&content),
        &spectator_standing(),
    )
}

/// A board hung from a real world, and the config behind it.
fn floor(view: &WorldView) -> (Board, GameData, MarketConfig) {
    let data = data();
    let market = data.config.market.clone();
    let mut board = Board::new();
    board.refresh(&Snapshot::read(view), &data.instruments, &market);
    (board, data, market)
}

/// The deepest listing on the board, so the fixtures are not fighting slippage
/// when they are testing something else.
fn liquid(board: &Board) -> Instrument {
    board
        .listings()
        .max_by(|a, b| a.liquidity.total_cmp(&b.liquidity))
        .expect("the board is not empty")
        .clone()
}

#[test]
fn a_round_trip_at_an_unchanged_price_loses_exactly_the_friction() {
    // The clearest statement of what trading costs: buy and sell back with the
    // world standing still, and the loss is slippage and fees, nothing else.
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);
    let opened = ledger.cash;

    let bought = ledger.execute(&listing, Side::Buy, 20, 1, orders).unwrap();
    let sold = ledger.execute(&listing, Side::Sell, 20, 1, orders).unwrap();

    assert_eq!(ledger.units_of(&listing.entity), 0);
    assert!(ledger.position(&listing.entity).is_none(), "and is closed");

    let lost = opened - ledger.cash;
    assert!(lost > 0.0, "a round trip is never free: {lost}");
    assert!(
        (lost - (bought.friction() + sold.friction())).abs() < 0.05,
        "lost {lost} against friction {} + {}",
        bought.friction(),
        sold.friction()
    );
    // And the realised figure agrees with the cash, which is the invariant that
    // would catch a basis leak.
    assert!((ledger.realised + lost).abs() < 0.05, "{}", ledger.realised);
}

#[test]
fn the_exchange_does_not_lend_paper_or_money() {
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);

    // No short selling.
    assert_eq!(
        ledger.execute(&listing, Side::Sell, 1, 1, orders),
        Err(OrderError::Short { held: 0 })
    );

    // And no buying on credit. The size is chosen to cost far more than the
    // opening capital whatever the listing's price.
    let huge = (orders.starting_capital / listing.price.max(0.01)) as i64 * 10 + 1000;
    let refused = ledger.execute(&listing, Side::Buy, huge, 1, orders);
    assert!(
        matches!(refused, Err(OrderError::Insufficient { .. })),
        "{refused:?}"
    );
    assert_eq!(
        ledger.cash, orders.starting_capital,
        "and nothing was spent"
    );

    assert_eq!(
        ledger.execute(&listing, Side::Buy, 0, 1, orders),
        Err(OrderError::NoUnits)
    );
}

#[test]
fn a_partial_sale_takes_its_share_of_the_basis_and_no_more() {
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);

    ledger.execute(&listing, Side::Buy, 40, 1, orders).unwrap();
    let basis = ledger.position(&listing.entity).unwrap().cost_basis;
    let average = ledger.position(&listing.entity).unwrap().average_cost();

    ledger.execute(&listing, Side::Sell, 10, 2, orders).unwrap();
    let held = ledger.position(&listing.entity).unwrap();

    assert_eq!(held.units, 30);
    assert!(
        (held.cost_basis - basis * 0.75).abs() < 0.01,
        "three quarters of the basis stays with three quarters of the units"
    );
    // Average cost is unchanged by a partial sale, which is the point of using
    // it rather than lots: what you realise cannot depend on which imaginary
    // certificate you handed over.
    assert!((held.average_cost() - average).abs() < 0.01);
}

#[test]
fn fees_are_part_of_what_a_position_cost() {
    // A P&L that ignores what the trading cost is a P&L that flatters the
    // trader, so the fee joins the basis rather than vanishing into a footnote.
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);

    let fill = ledger.execute(&listing, Side::Buy, 25, 1, orders).unwrap();
    let position = ledger.position(&listing.entity).unwrap();

    assert!((position.cost_basis - (fill.gross + fill.fee)).abs() < 0.01);
    assert!(position.average_cost() > fill.price, "the fee is in there");
    assert_eq!(ledger.fees_paid, fill.fee);
    // So a position bought and immediately valued at the screen price is
    // already down by the friction, which is true and worth showing.
    assert!(position.unrealised(listing.price) < 0.0);
}

#[test]
fn a_coupon_is_paid_on_every_unit_held() {
    let view = world();
    let (board, data, _) = floor(&view);
    let orders = &data.config.orders;
    let paying = board
        .listings()
        .find(|listing| listing.income > 0.0)
        .expect("a province pays a coupon")
        .clone();
    let mut ledger = Portfolio::new(orders);
    ledger.execute(&paying, Side::Buy, 12, 1, orders).unwrap();
    let before = ledger.cash;

    let paid = ledger.collect_income(&board);

    assert!((paid - paying.income * 12.0).abs() < 0.01);
    assert!((ledger.cash - before - paid).abs() < 0.01);
    assert_eq!(ledger.income_received, paid);
}

#[test]
fn a_delisted_position_is_a_total_loss_rather_than_a_markdown() {
    // The risk that makes emergent delistings matter (GDD 8): when a town is
    // razed, the paper is not marked down, it is worthless.
    let view = world();
    let (mut board, data, market) = floor(&view);
    let orders = &data.config.orders;
    let town = board
        .listings()
        .find(|listing| listing.class == "Settlement share")
        .expect("a town is listed")
        .clone();
    let mut ledger = Portfolio::new(orders);
    ledger.execute(&town, Side::Buy, 15, 1, orders).unwrap();
    let basis = ledger.position(&town.entity).unwrap().cost_basis;
    let cash = ledger.cash;

    // The world razes it.
    let mut razed = view.clone();
    razed.settlements.retain(|s| s.id != town.entity.id());
    board.refresh(&Snapshot::read(&razed), &data.instruments, &market);

    let struck = ledger.write_off_delisted(&board);

    assert_eq!(struck, vec![town.symbol.clone()]);
    assert!(ledger.position(&town.entity).is_none());
    assert!(
        (ledger.realised + basis).abs() < 0.01,
        "the whole basis is lost"
    );
    assert_eq!(ledger.cash, cash, "and no cash came back");
    // Equity fell by exactly what the position was worth on the board.
    assert_eq!(ledger.equity(&board), ledger.cash);
}

#[test]
fn equity_is_cash_plus_what_the_board_says_the_holdings_are_worth() {
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);
    ledger.execute(&listing, Side::Buy, 30, 1, orders).unwrap();

    let expected = ledger.cash + listing.price * 30.0;
    assert!((ledger.equity(&board) - expected).abs() < 0.01);
    assert!((ledger.holdings_value(&board) - listing.price * 30.0).abs() < 0.01);
    // Down by the friction the moment it is bought, and unrealised says so.
    assert!(ledger.unrealised(&board) < 0.0);
    assert!(ledger.equity(&board) < orders.starting_capital);
}

#[test]
fn a_ledger_survives_a_save_with_its_basis_intact() {
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);
    ledger.execute(&listing, Side::Buy, 18, 1, orders).unwrap();
    ledger.collect_income(&board);

    let resumed = Portfolio::from_save(&ledger.to_save(), orders);

    assert_eq!(resumed.cash, ledger.cash);
    assert_eq!(resumed.units_of(&listing.entity), 18);
    assert_eq!(
        resumed.position(&listing.entity).unwrap().cost_basis,
        ledger.position(&listing.entity).unwrap().cost_basis
    );
    assert_eq!(resumed.fees_paid, ledger.fees_paid);
    assert_eq!(resumed.income_received, ledger.income_received);
    assert_eq!(resumed.equity(&board), ledger.equity(&board));
}

#[test]
fn a_save_written_before_there_were_positions_opens_a_fresh_ledger() {
    let orders = &data().config.orders;
    let resumed = Portfolio::from_save(&PortfolioSave::default(), orders);

    assert_eq!(resumed.cash, orders.starting_capital);
    assert_eq!(resumed.opening_capital, orders.starting_capital);
}

#[test]
fn the_tape_records_what_was_traded() {
    let (board, data, _) = floor(&world());
    let listing = liquid(&board);
    let orders = &data.config.orders;
    let mut ledger = Portfolio::new(orders);

    ledger.execute(&listing, Side::Buy, 5, 1, orders).unwrap();
    ledger.execute(&listing, Side::Buy, 7, 2, orders).unwrap();

    assert_eq!(ledger.ledger().len(), 2);
    assert_eq!(ledger.ledger()[0].units, 7, "newest first");
    assert_eq!(ledger.ledger()[0].year, 2);
}
