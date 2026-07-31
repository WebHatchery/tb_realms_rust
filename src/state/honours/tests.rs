//! What the exchange notices, and what it must not notice twice.

use super::*;
use crate::data::GameData;
use crate::market::orders::Side;
use crate::world::Snapshot;
use macroquad_toolkit::achievements::Achievements;
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

fn floor() -> (Board, GameData) {
    let data = data();
    let mut board = Board::new();
    board.refresh(
        &Snapshot::read(&world()),
        &data.instruments,
        &data.config.market,
        1,
    );
    (board, data)
}

fn honours(data: &GameData) -> Achievements {
    Achievements::from_definitions(data.achievements.clone())
}

/// A standing with nothing remarkable about it.
fn quiet<'a>(portfolio: &'a Portfolio, board: &'a Board) -> Standing<'a> {
    Standing {
        portfolio,
        board,
        licences: 0,
        observed: 1,
        cataclysm: false,
        written_off: 0,
        income: 0.0,
    }
}

#[test]
fn the_shipped_definitions_cover_every_rule_that_can_fire() {
    // A rule awarding an id no definition carries would unlock nothing and say
    // nothing — the achievement would simply never appear.
    let data = data();
    let mut honours = honours(&data);
    let (board, _) = floor();
    let portfolio = Portfolio::new(&data.config.orders);

    // Everything true at once, so every rule in `review` fires.
    let standing = Standing {
        portfolio: &portfolio,
        board: &board,
        licences: 3,
        observed: 1_000,
        cataclysm: true,
        written_off: 1,
        income: 10.0,
    };
    let won = review(&mut honours, &standing);

    assert!(!won.is_empty());
    for id in &won {
        assert!(
            honours.get(id).is_some(),
            "{id} is awarded but never defined"
        );
    }
}

#[test]
fn an_honour_is_won_once_and_never_again() {
    let data = data();
    let mut honours = honours(&data);
    let (board, _) = floor();
    let portfolio = Portfolio::new(&data.config.orders);

    let mut standing = quiet(&portfolio, &board);
    standing.income = 5.0;

    let first = review(&mut honours, &standing);
    assert!(first.contains(&"first_coupon".to_owned()));

    // The same standing a year later must announce nothing.
    let again = review(&mut honours, &standing);
    assert!(
        !again.contains(&"first_coupon".to_owned()),
        "announced twice: {again:?}"
    );
}

#[test]
fn surviving_an_age_requires_both_surviving_and_being_up() {
    // The game's signature achievement (GDD 5.6): it is not enough to be there.
    let data = data();
    let (board, _) = floor();
    let mut portfolio = Portfolio::new(&data.config.orders);

    // Down on the year: the age ends and the broker does not get the credit.
    portfolio.cash = portfolio.opening_capital - 1.0;
    let mut poorer = honours(&data);
    let mut standing = quiet(&portfolio, &board);
    standing.cataclysm = true;
    let survived = review(&mut poorer, &standing);
    assert!(survived.contains(&"age_witness".to_owned()), "still saw it");
    assert!(!survived.contains(&"age_survivor".to_owned()));

    // Up on the year, through the same ending.
    portfolio.cash = portfolio.opening_capital + 1.0;
    let mut richer = honours(&data);
    let standing = Standing {
        portfolio: &portfolio,
        ..quiet(&portfolio, &board)
    };
    let standing = Standing {
        cataclysm: true,
        ..standing
    };
    assert!(review(&mut richer, &standing).contains(&"age_survivor".to_owned()));
}

#[test]
fn the_spread_book_counts_classes_and_not_positions() {
    // Four positions in four towns is one class, and not a spread book.
    let data = data();
    let (board, _) = floor();
    let orders = &data.config.orders;
    let mut portfolio = Portfolio::new(orders);

    let towns: Vec<_> = board
        .listings()
        .filter(|l| l.class == "Settlement share")
        .take(4)
        .cloned()
        .collect();
    assert_eq!(towns.len(), 4, "the fixture needs four towns");
    for town in &towns {
        portfolio.execute(town, Side::Buy, 1, 1, orders).unwrap();
    }

    let mut honours = honours(&data);
    assert!(!review(&mut honours, &quiet(&portfolio, &board)).contains(&"diversified".to_owned()));

    // A fourth *class* is what earns it.
    for class in ["Regional bond", "Route toll", "Commodity"] {
        let listing = board
            .listings()
            .find(|l| l.class == class)
            .expect("class is listed")
            .clone();
        portfolio
            .execute(&listing, Side::Buy, 1, 1, orders)
            .unwrap();
    }
    assert!(review(&mut honours, &quiet(&portfolio, &board)).contains(&"diversified".to_owned()));
}

#[test]
fn a_big_order_teaches_a_lesson_about_size() {
    let data = data();
    let (board, _) = floor();
    let orders = &data.config.orders;
    let mut portfolio = Portfolio::new(orders);

    // The thinnest paper on the board, bought in bulk.
    let thin = board
        .listings()
        .min_by(|a, b| a.liquidity.total_cmp(&b.liquidity))
        .expect("something is listed")
        .clone();
    let units = (orders.depth(thin.liquidity) * 0.5) as i64;
    let before = portfolio.realised;
    let fill = portfolio
        .execute(&thin, Side::Buy, units.max(1), 1, orders)
        .expect("affordable");

    let mut honours = honours(&data);
    let won = review_fill(&mut honours, &fill, &board, before, portfolio.realised);

    assert!(won.contains(&"first_ticket".to_owned()));
    assert!(
        won.contains(&"size_lesson".to_owned()),
        "slippage was {}",
        fill.slippage
    );
}

#[test]
fn a_sale_that_made_nothing_is_not_a_round_trip() {
    let data = data();
    let (board, _) = floor();
    let orders = &data.config.orders;
    let mut portfolio = Portfolio::new(orders);
    let listing = board
        .listings()
        .max_by(|a, b| a.liquidity.total_cmp(&b.liquidity))
        .expect("something is listed")
        .clone();

    portfolio
        .execute(&listing, Side::Buy, 10, 1, orders)
        .unwrap();
    let before = portfolio.realised;
    let fill = portfolio
        .execute(&listing, Side::Sell, 10, 2, orders)
        .unwrap();

    // Bought and sold at an unchanged price: the friction guarantees a loss.
    assert!(portfolio.realised < before);
    let mut honours = honours(&data);
    let won = review_fill(&mut honours, &fill, &board, before, portfolio.realised);
    assert!(!won.contains(&"round_trip".to_owned()), "{won:?}");
}
