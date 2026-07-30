//! How a price forms, and why it moved (GDD 5.4).
//!
//! **Price is not the fundamental.** If it were, the game would be arithmetic
//! and there would be nothing to be right about. Each settlement, a price is
//! nudged by four terms:
//!
//! ```text
//! drift  = mean reversion toward fair value      // what the world says it's worth
//! shock  = event impact from typed state deltas  // what just happened to it
//! noise  = seeded random walk on class volatility// what the floor does anyway
//! flow   = the player's own accumulated orders   // what you did to it yourself
//! price' = price + drift + shock + noise + flow
//! ```
//!
//! Skill is estimating fair value faster and better than the drift can close the
//! gap. Every term survives into [`PriceMove`] so a listing screen can always
//! answer "why did it move" with a decomposition rather than a shrug — Pillar 1
//! requires that, and it is why the terms here are **rescaled rather than
//! truncated** when a move is capped: after the cap, they still sum exactly to
//! the move that happened. A decomposition whose parts do not add up is worse
//! than none.
//!
//! No macroquad. A century of settlements runs headlessly through this.

use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};

/// How prices move, as `game_config.json` states it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConfig {
    /// The market's own noise stream. Saved and shown on the records screen:
    /// same seed and same orders means the same price history, which is what
    /// makes the determinism and backtest tests of GDD 14 possible.
    pub seed: u64,
    /// The fraction of the gap to fair value a price closes each settlement.
    /// Low enough that a well-read broker is paid for being early; high enough
    /// that the market is not permanently wrong.
    pub mean_reversion: f32,
    /// Multiplier on each class's own volatility.
    pub noise_scale: f32,
    /// The most a price may move in one settlement, as a fraction of itself.
    /// A cataclysm should hurt, not teleport.
    pub max_move: f32,
    /// No price falls below this. A claim on something that still exists is
    /// never worth exactly nothing — and a price of zero would make every
    /// fraction-of-price term below dead arithmetic forever after.
    pub min_price: f32,
    /// Settlements of price history kept per listing. `series` never grows and
    /// never loses an extreme, so this is a resolution budget, not a horizon.
    pub history_capacity: usize,
}

/// Why a price moved, in the terms of GDD 5.4.
///
/// The parts sum to `to - from`, always — including when the move was capped.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PriceMove {
    pub from: f32,
    /// Mean reversion toward fair value.
    pub drift: f32,
    /// The jolt from what the world just did, per `impacts.json`.
    pub shock: f32,
    /// The floor's own restlessness.
    pub noise: f32,
    /// GDD 5.4 calls this term `impact`: the player's own order flow. It is
    /// named `flow` here because `impact.rs` next door produces `shock`, and two
    /// things called impact in one decomposition is how a decomposition stops
    /// being readable.
    pub flow: f32,
    pub to: f32,
}

impl PriceMove {
    /// A listing that has not moved yet — newly listed, and priced fairly.
    pub fn unmoved(price: f32) -> Self {
        Self {
            from: price,
            to: price,
            ..Default::default()
        }
    }

    pub fn change(&self) -> f32 {
        self.to - self.from
    }

    /// The move as a fraction of where it started, which is how a board reads.
    pub fn change_fraction(&self) -> f32 {
        if self.from.abs() < f32::EPSILON {
            return 0.0;
        }
        self.change() / self.from
    }

    /// The terms, largest first, for the decomposition view. Terms that
    /// contributed nothing are dropped — a listing that only drifted should say
    /// so, not list three zeroes.
    pub fn parts(&self) -> Vec<(&'static str, f32)> {
        let mut parts: Vec<(&'static str, f32)> = [
            ("drift", self.drift),
            ("shock", self.shock),
            ("noise", self.noise),
            ("flow", self.flow),
        ]
        .into_iter()
        .filter(|(_, value)| value.abs() > f32::EPSILON)
        .collect();
        parts.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
        parts
    }
}

/// Settle one price for one turning of the world.
///
/// `shock` and `flow` arrive as fractions of price; the class's `volatility`
/// scales the noise the same way. Everything is proportional so that a relic at
/// 900 crowns and a hamlet at 4 behave alike in percentage terms, which is the
/// only sense in which they are comparable.
pub fn settle(
    price: f32,
    fair_value: f32,
    volatility: f32,
    shock: f32,
    flow: f32,
    config: &MarketConfig,
    rng: &mut SeededRng,
) -> PriceMove {
    let drift = (fair_value - price) * config.mean_reversion;
    let noise = price * volatility * config.noise_scale * bell(rng);
    let shock = price * shock;
    let flow = price * flow;

    // What the terms want, what the cap allows, and what the floor leaves.
    let wanted = drift + shock + noise + flow;
    let cap = (price * config.max_move).abs();
    let allowed = wanted.clamp(-cap, cap);
    let to = (price + allowed).max(config.min_price);

    // Rescale so the decomposition still adds up to the move that happened —
    // but only when the cap or the floor actually bit. An unconstrained move
    // needs no rescaling, and dividing by a ratio that ought to be exactly one
    // would put rounding error into every term for nothing.
    let scale = if allowed == wanted && to == price + allowed {
        1.0
    } else if wanted.abs() > f32::EPSILON {
        (to - price) / wanted
    } else {
        0.0
    };

    PriceMove {
        from: price,
        drift: drift * scale,
        shock: shock * scale,
        noise: noise * scale,
        flow: flow * scale,
        to,
    }
}

/// A draw in `[-1, 1]`, humped in the middle.
///
/// Two uniforms summed is a triangular distribution — not a Gaussian, but zero
/// mean, bounded, and one line. Bounded matters: an unbounded tail would let the
/// noise term alone produce a move the cap then has to swallow, and the cap
/// exists for cataclysms, not for arithmetic.
fn bell(rng: &mut SeededRng) -> f32 {
    rng.next_f32() + rng.next_f32() - 1.0
}

#[cfg(test)]
mod tests;
