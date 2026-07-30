//! The exchange's own half: what is listed, what it is worth, and — from M2 —
//! what it currently trades at (GDD 5.2, 5.4, 11).
//!
//! **Nothing under `market/` may depend on macroquad.** The headline test of
//! GDD 14 runs a full century of settlements headlessly with a scripted strategy
//! and asserts the equity curve stays in bounds; that only works while this
//! layer is engine-free. It reads a [`crate::world::Snapshot`] and returns
//! numbers, and it never draws.
//!
//! The layers, bottom up: `instruments` derives the board and each listing's
//! fair value from the world; `impact` turns typed world deltas into shocks;
//! `pricing` moves each price by drift, shock, noise and the player's own order
//! flow. Nothing above the seam invents a number.

pub mod impact;
pub mod instruments;
pub mod pricing;

pub use impact::ImpactTable;
pub use instruments::{Board, BoardChange, Instrument, InstrumentRules};
pub use pricing::MarketConfig;

#[cfg(test)]
mod tests;
