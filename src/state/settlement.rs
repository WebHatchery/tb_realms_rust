//! One turning of the world, as the exchange saw it.
//!
//! A settlement record is the digest of a single year: what the world did, what
//! that did to the board, and — beside it — the chronicle's own words for the
//! same year. The two are recorded together and *nothing joins them but the
//! year*, because the price oracle never reads prose (GDD 5.5). Any
//! correspondence a reader sees between them is one they inferred.

use crate::market::{Board, BoardChange, Instrument};
use crate::world::delta::{Change, WorldDelta};
use crate::world::fundamentals::EntityRef;
use serde::{Deserialize, Serialize};

/// One turning of the world, as the exchange saw it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub year: u32,
    pub tick: u64,
    /// World years covered — more than one if the exchange was off the wire.
    pub years: u32,
    pub moves: usize,
    pub listed: usize,
    pub delisted: usize,
    /// Listings whose fair value moved this turning.
    pub revalued: usize,
    /// A few of the instruments that appeared or vanished, by ticker. This is
    /// the content no authored market can produce — a house whose blood ran
    /// out, a settlement razed — so the digest says which, not merely how many.
    pub notable: Vec<String>,
    /// The fields that moved furthest, in the vocabulary the balance tables
    /// will use. The embryo of the price-move decomposition (GDD 5.4): a
    /// player must always be able to ask why, and get an answer.
    pub top_moves: Vec<String>,
    /// The chronicle's own words for this turning, if it had any: the newspaper
    /// beside the numbers (GDD 5.5). Never read by anything that prices.
    pub headline: Option<String>,
    /// What actually moved that year, largest first — the numbers to read the
    /// prose against.
    pub movers: Vec<Mover>,
    /// An age ended in this settlement. Set by the session, which is the only
    /// thing that knows what era the previous reading was in.
    #[serde(default)]
    pub cataclysm: bool,
}

/// How many appearances/disappearances a single record names before it stops.
const NOTABLE_LIMIT: usize = 3;

/// How many of a settlement's biggest price moves a record names.
const MOVER_LIMIT: usize = 6;

/// What moved furthest this settlement, largest first.
///
/// This is the honest cross-link between the chronicle and the market. The
/// prose is never parsed for signal (GDD 5.5) — instead a year's *prose* and a
/// year's *price moves* are recorded side by side, and the reader draws their
/// own conclusions. When the two disagree, that gap is the point.
pub(super) fn movers(board: &Board) -> Vec<Mover> {
    let mut moved: Vec<&Instrument> = board
        .listings()
        .filter(|listing| listing.last_move.change() != 0.0)
        .collect();
    moved.sort_by(|a, b| {
        b.last_move
            .change_fraction()
            .abs()
            .total_cmp(&a.last_move.change_fraction().abs())
    });
    moved
        .into_iter()
        .take(MOVER_LIMIT)
        .map(|listing| Mover {
            symbol: listing.symbol.clone(),
            entity: listing.entity.clone(),
            change: listing.last_move.change_fraction(),
        })
        .collect()
}

/// One listing that moved, named so the chronicle can be read against it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mover {
    pub symbol: String,
    pub entity: EntityRef,
    /// The move as a fraction of where the price started.
    pub change: f32,
}

/// A discrete change in words — the direction lives in the labels rather than
/// in a magnitude, so it has to be spelled out.
fn describe(change: &Change) -> String {
    match change {
        Change::Scalar { before, after } => format!("{before:.1} → {after:.1}"),
        Change::Flag { after: true, .. } => "set".to_owned(),
        Change::Flag { after: false, .. } => "lifted".to_owned(),
        Change::Status { before, after } => format!("{before} → {after}"),
    }
}

impl SettlementRecord {
    pub(super) fn from_delta(delta: &WorldDelta, board: &BoardChange, movers: Vec<Mover>) -> Self {
        // Delistings lead: a position in something that no longer exists is the
        // more urgent news than a new listing to consider. These are the board's
        // own tickers rather than the delta's raw entities, because a resource
        // node appearing is supply news, not a security anyone can hold.
        let notable = board
            .delisted
            .iter()
            .map(|symbol| format!("{symbol} delisted"))
            .chain(board.listed.iter().map(|symbol| format!("{symbol} listed")))
            .take(NOTABLE_LIMIT)
            .collect();

        let top_moves = delta
            .most_moved(NOTABLE_LIMIT)
            .into_iter()
            .map(|moved| {
                let magnitude = moved.change.magnitude();
                if magnitude == 0.0 {
                    // A flag or status change: the direction is in the labels,
                    // not in a number, so print the labels.
                    format!("{} {}", moved.entity.id(), describe(&moved.change))
                } else {
                    format!(
                        "{} {} {:+.1}",
                        moved.entity.id(),
                        moved.field.key(),
                        magnitude
                    )
                }
            })
            .collect();

        Self {
            year: delta.to_year,
            tick: delta.to_tick,
            years: delta.years(),
            moves: delta.moves.len(),
            listed: board.listed.len(),
            delisted: board.delisted.len(),
            revalued: board.revalued,
            notable,
            top_moves,
            headline: delta.events.last().map(|event| event.message.clone()),
            movers,
            cataclysm: false,
        }
    }
}
