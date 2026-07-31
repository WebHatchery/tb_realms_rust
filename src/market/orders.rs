//! Execution: what an order actually costs (GDD 5.5, and open question 3).
//!
//! **Size is a cost.** A broker who could buy any quantity at the screen price
//! would find capital had no ceiling and the game no shape, so an order pays
//! for its own size twice, in two different ways:
//!
//! - **Slippage**, immediately: the fill is worse than the quote, in proportion
//!   to how much of the listing's depth the order eats. Temporary, and paid at
//!   the moment of trading.
//! - **Flow**, afterwards: the order leaves a mark on the price itself, arriving
//!   as GDD 5.4's fifth term at the next settlement and decaying from there.
//!   Permanent-ish, and paid by everyone including the broker who caused it.
//!
//! Depth comes from the class's `liquidity`, which is why a regional bond
//! absorbs a large order and a relic lot does not. GDD 5.2 promises relics
//! "enormous spreads"; this is where that promise is kept.
//!
//! [`quote`] is used for both the estimate on the ticket and the execution
//! itself, so the number a player is shown before trading is the number they
//! get. Two code paths there would be a way to lie to them.
//!
//! No macroquad: the backtest harness executes through this file.

use crate::market::Instrument;
use crate::world::fundamentals::EntityRef;
use serde::{Deserialize, Serialize};

/// The fee schedule and impact model, as `game_config.json` states it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfig {
    /// What a broker opens the ledger with.
    pub starting_capital: f32,
    /// Commission as a fraction of the order's value.
    pub commission: f32,
    /// The least any order pays, so a stream of tiny trades is not free.
    pub minimum_fee: f32,
    /// Units of the most liquid conceivable listing the floor absorbs without
    /// flinching. A class's own depth is this scaled by its `liquidity`.
    pub depth_units: f32,
    /// How hard eating the depth moves the fill against you.
    pub slippage_scale: f32,
    /// The worst fill slippage may produce, however large the order.
    pub max_slippage: f32,
    /// How much of an order's size persists into the price as flow.
    pub flow_scale: f32,
    /// The fraction of accumulated flow that survives each settlement. The
    /// market forgets a large order gradually, not instantly.
    pub flow_decay: f32,
}

impl OrderConfig {
    /// How many units of a listing the floor will take before the price starts
    /// to argue. Never zero, so the arithmetic below cannot divide by it.
    pub fn depth(&self, liquidity: f32) -> f32 {
        (self.depth_units * liquidity).max(1.0)
    }
}

/// Which way an order goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Side::Buy => "Buy",
            Side::Sell => "Sell",
        }
    }

    /// The sign this side applies to a position and to order flow.
    pub fn sign(self) -> f32 {
        match self {
            Side::Buy => 1.0,
            Side::Sell => -1.0,
        }
    }
}

/// What an order did, or would do — the same struct either way.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub symbol: String,
    pub entity: EntityRef,
    pub side: Side,
    pub units: i64,
    /// The quoted price before the order's own size was accounted for.
    pub quoted: f32,
    /// What each unit actually cost, slippage included.
    pub price: f32,
    /// Slippage as a fraction of the quote. Always against the trader.
    pub slippage: f32,
    pub fee: f32,
    /// Units times the fill price, before fees.
    pub gross: f32,
    /// The signed change to the ledger: negative when buying.
    pub cash: f32,
    /// The mark this order leaves on the price, as a fraction, arriving at the
    /// next settlement as GDD 5.4's `flow` term.
    pub flow: f32,
    /// The world year the order was struck.
    pub year: u32,
}

impl Fill {
    /// What the order cost above the quoted price — slippage and fees together.
    /// The number that tells a player their size was the problem.
    pub fn friction(&self) -> f32 {
        let quoted = self.quoted * self.units as f32;
        (self.gross - quoted).abs() + self.fee
    }
}

/// Price an order without applying it.
///
/// `units` is a magnitude; the direction is in `side`. Zero or negative units
/// produce a fill of nothing, which the portfolio rejects rather than executes.
pub fn quote(
    listing: &Instrument,
    side: Side,
    units: i64,
    year: u32,
    config: &OrderConfig,
) -> Fill {
    let units = units.max(0);
    let size = units as f32;
    let depth = config.depth(listing.liquidity);

    // Eating the depth costs you. Capped, because past a point the honest
    // answer is that the order is simply too big — which the cap makes visible
    // as a fill that stops improving no matter how the size is sliced.
    let slippage = ((size / depth) * config.slippage_scale).min(config.max_slippage);
    // Slippage is always against the trader: dearer to buy, cheaper to sell.
    let price = (listing.price * (1.0 + slippage * side.sign())).max(0.0);
    let gross = price * size;
    let fee = if units == 0 {
        0.0
    } else {
        (gross * config.commission).max(config.minimum_fee)
    };

    Fill {
        symbol: listing.symbol.clone(),
        entity: listing.entity.clone(),
        side,
        units,
        quoted: listing.price,
        price,
        slippage,
        fee,
        gross,
        cash: -side.sign() * gross - fee,
        flow: (size / depth) * config.flow_scale * side.sign(),
        year,
    }
}

#[cfg(test)]
mod tests;
