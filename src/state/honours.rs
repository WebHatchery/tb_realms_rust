//! What a broker has earned, and the one place that decides it.
//!
//! Achievements are checked from the session's own facts — the ledger, the
//! board, the licence ladder, the settlement just recorded — rather than being
//! unlocked from a dozen scattered call sites. One function, run once per
//! settlement and once per fill, means an achievement cannot be awarded twice by
//! two paths that both thought they were responsible, and a new one is a rule
//! added here rather than a hook threaded through the game.
//!
//! Nothing here rewards the player with anything but the record of it. There is
//! no currency, no unlock, no bonus: the exchange notices, and that is all. A
//! reward would make it a second progression axis competing with the licences,
//! which are meant to be the only one (Pillar 3).

use crate::market::orders::Fill;
use crate::market::Board;
use crate::state::portfolio::Portfolio;
use macroquad_toolkit::achievements::Achievements;

/// Everything the checks below need to know, gathered once by the caller.
///
/// A struct rather than a long argument list because these are read in almost
/// every rule and the compiler should complain when one is forgotten, not when
/// the argument order is quietly wrong.
pub struct Standing<'a> {
    pub portfolio: &'a Portfolio,
    pub board: &'a Board,
    /// Rungs of the information ladder held.
    pub licences: usize,
    /// Settlements observed this session.
    pub observed: u64,
    /// An age ended in the settlement just recorded.
    pub cataclysm: bool,
    /// Positions written off because their paper was struck from the board.
    pub written_off: usize,
    /// Coupons paid this settlement.
    pub income: f32,
}

/// Award whatever this standing has earned, and return what was newly won.
///
/// Returns only *new* unlocks, so a caller may announce them without having to
/// remember what it announced last time.
pub fn review(honours: &mut Achievements, standing: &Standing<'_>) -> Vec<String> {
    let portfolio = standing.portfolio;
    let equity = portfolio.equity(standing.board);
    let opening = portfolio.opening_capital;

    // Positions held, by the class of paper they are — the spread book.
    let classes: std::collections::BTreeSet<&str> = portfolio
        .positions()
        .filter_map(|position| position.entity.listing_class())
        .collect();

    let earned = [
        ("first_coupon", standing.income > 0.0),
        ("chartered", standing.licences >= 1),
        ("guild_factor", standing.licences >= 2),
        ("archivist", standing.licences >= 3),
        ("delisted", standing.written_off > 0),
        ("age_witness", standing.cataclysm),
        // Solvent through the ending: the game's signature achievement
        // (GDD 5.6). Checked only on the settlement an age actually ended.
        ("age_survivor", standing.cataclysm && equity > opening),
        ("century", standing.observed >= 100),
        ("doubled", opening > 0.0 && equity >= opening * 2.0),
        ("diversified", classes.len() >= 4),
    ];

    let mut won = Vec::new();
    for (id, achieved) in earned {
        if achieved && honours.unlock(id) {
            won.push(id.to_owned());
        }
    }
    won
}

/// Award whatever an order has earned.
///
/// Separate from [`review`] because a fill is not a settlement: these are facts
/// about the trade itself, and the trade has already been priced by the time
/// anything else could look at it.
pub fn review_fill(
    honours: &mut Achievements,
    fill: &Fill,
    board: &Board,
    realised_before: f32,
    realised_after: f32,
) -> Vec<String> {
    let premium = board
        .get(&fill.entity)
        .map(|listing| listing.premium())
        .unwrap_or(0.0);

    let earned = [
        ("first_ticket", true),
        // Bought a fifth or more under what the world says it is worth. The
        // whole game in one order, if the fundamentals were read right.
        (
            "value_broker",
            fill.side == crate::market::orders::Side::Buy && premium < -0.20,
        ),
        // Slippage alone cost more than a tenth of the order: your size was the
        // problem, and the exchange would like you to know it.
        ("size_lesson", fill.slippage > 0.10),
        // A closed position that actually made money once the floor had taken
        // its cut, which is a much rarer thing than a position that went up.
        (
            "round_trip",
            fill.side == crate::market::orders::Side::Sell && realised_after > realised_before,
        ),
    ];

    let mut won = Vec::new();
    for (id, achieved) in earned {
        if achieved && honours.unlock(id) {
            won.push(id.to_owned());
        }
    }
    won
}

#[cfg(test)]
mod tests;
