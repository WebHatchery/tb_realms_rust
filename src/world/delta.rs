//! Typed differences between two consecutive world views (GDD 5.5).
//!
//! **The price oracle reads numbers, never prose.** Mytherra's `WorldEvent` is
//! `{ year, kind, message }` with four coarse kinds and free text for substance;
//! parsing that text for market signal would break silently the first time a
//! string was reworded. So the exchange diffs the *state* between consecutive
//! [`WorldView`]s and prices the result: a region's `harvest` fell thirty
//! points and `famine` flipped true, a trade route's `volume` halved, a resource
//! node's status changed, a house was founded or its line ran out.
//!
//! The chronicle rides alongside in [`WorldDelta::events`] as the newspaper —
//! rendered for the player, never read by the pricer. That split is what makes
//! GDD Pillar 4 mechanical: the prose is flavour and the deltas are truth.
//!
//! The diff itself is a comparison of two [`Snapshot`]s, which is the same
//! reading of the world the pricer values (GDD 5.4). Reading the world once and
//! using it for both is what keeps a listing's decomposition adding up.
//!
//! Everything here is pure. No macroquad, no I/O, no engine types — so a
//! century of deltas can be replayed headlessly by the backtest harness
//! (GDD 14).

use crate::world::fundamentals::{Fundamentals, Snapshot, Value};
use mytherra_core::world::WorldEvent;
use mytherra_protocol::WorldView;

pub use crate::world::fundamentals::{EntityRef, Field};

/// Scalar moves smaller than this are treated as noise and dropped.
///
/// The world's stats are `f32` and several are recomputed from sums each tick,
/// so an untouched value can wobble in its last bits. A change below this
/// threshold is not something a market could price anyway, and admitting it
/// would fill every settlement's decomposition with meaningless rows.
const SCALAR_EPSILON: f32 = 0.001;

/// How a watched field moved.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A continuous value moved. Both ends are kept so a pricing rule can use
    /// the difference, the ratio, or a threshold crossing without re-deriving
    /// any of them.
    Scalar { before: f32, after: f32 },
    /// A flag flipped — famine set or lifted.
    Flag { before: bool, after: bool },
    /// A discrete status changed, as its world variant name.
    Status { before: String, after: String },
}

impl Change {
    /// The signed size of a scalar move; zero for the discrete kinds, which
    /// carry direction in their labels rather than in a magnitude.
    pub fn magnitude(&self) -> f32 {
        match self {
            Change::Scalar { before, after } => after - before,
            Change::Flag { .. } | Change::Status { .. } => 0.0,
        }
    }
}

/// One field of one entity moving between consecutive views.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldMove {
    pub entity: EntityRef,
    pub field: Field,
    pub change: Change,
}

/// Everything that changed between two consecutive world views.
///
/// `listed` and `delisted` are the ones no authored market could produce: a
/// house is founded when a hero passes into legend, and delisted when its blood
/// runs out; a settlement can be razed outright. An instrument whose backing
/// entity has left the world stops being tradeable and becomes a record
/// (GDD 8, 12 open question 6).
#[derive(Debug, Clone, Default)]
pub struct WorldDelta {
    pub to_tick: u64,
    pub from_year: u32,
    pub to_year: u32,
    /// Entities present in the new view and absent from the old.
    pub listed: Vec<EntityRef>,
    /// Entities present in the old view and absent from the new.
    pub delisted: Vec<EntityRef>,
    /// Field moves on entities present in both.
    pub moves: Vec<FieldMove>,
    /// The chronicle lines that accompany this turning — the newspaper, not the
    /// signal. Nothing in pricing may read these.
    pub events: Vec<WorldEvent>,
}

impl WorldDelta {
    /// How many world years this delta spans. Usually one; more if the exchange
    /// was away from the wire while the world kept turning.
    pub fn years(&self) -> u32 {
        self.to_year.saturating_sub(self.from_year)
    }

    /// Whether anything at all moved — a turning can be genuinely quiet.
    pub fn is_empty(&self) -> bool {
        self.listed.is_empty() && self.delisted.is_empty() && self.moves.is_empty()
    }

    /// The largest scalar moves first, for a digest that leads with what
    /// actually mattered rather than with whatever the diff happened to visit
    /// first. Discrete changes — a famine set, a mine contested — sort last by
    /// magnitude but are never dropped, because a flag flip can matter more
    /// than any number beside it.
    pub fn most_moved(&self, limit: usize) -> Vec<&FieldMove> {
        let mut ranked: Vec<&FieldMove> = self.moves.iter().collect();
        ranked.sort_by(|a, b| {
            b.change
                .magnitude()
                .abs()
                .total_cmp(&a.change.magnitude().abs())
        });
        ranked.truncate(limit);
        ranked
    }
}

/// Diff two consecutive world views into typed changes.
///
/// `before` and `after` need not be one tick apart; if the exchange was off the
/// wire for a decade the delta simply spans it, and `years()` reports how far.
pub fn diff(before: &WorldView, after: &WorldView) -> WorldDelta {
    diff_snapshots(&Snapshot::read(before), &Snapshot::read(after))
}

/// Diff two readings of the world. Separate from [`diff`] so a caller holding a
/// snapshot already — the board revalues from one every settlement — need not
/// read the same world twice.
pub fn diff_snapshots(before: &Snapshot, after: &Snapshot) -> WorldDelta {
    let mut delta = WorldDelta {
        to_tick: after.tick,
        from_year: before.year,
        to_year: after.year,
        ..Default::default()
    };

    for now in after.iter() {
        match before.get(&now.entity) {
            Some(was) => compare(was, now, &mut delta.moves),
            None => delta.listed.push(now.entity.clone()),
        }
    }
    for was in before.iter() {
        if after.get(&was.entity).is_none() {
            delta.delisted.push(was.entity.clone());
        }
    }

    delta
}

/// Every field of one entity that reads differently than it did.
///
/// Fields are compared by name rather than positionally, so a field the world
/// only sometimes carries simply produces no move instead of a spurious one.
fn compare(before: &Fundamentals, after: &Fundamentals, moves: &mut Vec<FieldMove>) {
    for (field, now) in &after.values {
        let Some(was) = before.values.get(field) else {
            continue;
        };
        let change = match (was, now) {
            (Value::Scalar(before), Value::Scalar(after)) => {
                if (after - before).abs() <= SCALAR_EPSILON {
                    continue;
                }
                Change::Scalar {
                    before: *before,
                    after: *after,
                }
            }
            (Value::Flag(before), Value::Flag(after)) if before != after => Change::Flag {
                before: *before,
                after: *after,
            },
            (Value::Status(before), Value::Status(after)) if before != after => Change::Status {
                before: before.clone(),
                after: after.clone(),
            },
            _ => continue,
        };
        moves.push(FieldMove {
            entity: after.entity.clone(),
            field: *field,
            change,
        });
    }
}

#[cfg(test)]
mod tests;
