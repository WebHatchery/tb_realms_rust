//! The exchange's own half: what is listed, what it is worth, and — from M2 —
//! what it currently trades at (GDD 5.2, 5.4, 11).
//!
//! **Nothing under `market/` may depend on macroquad.** The headline test of
//! GDD 14 runs a full century of settlements headlessly with a scripted strategy
//! and asserts the equity curve stays in bounds; that only works while this
//! layer is engine-free. It reads a [`crate::world::Snapshot`] and returns
//! numbers, and it never draws.
//!
//! Today this is M1: the taxonomy, fair value, and a board that lists and
//! delists as the world does. There is no price yet — a price is what M2 lets
//! drift toward the fair value these rules produce, and the gap between the two
//! is the whole game.

pub mod instruments;

pub use instruments::{Board, BoardChange, Instrument, InstrumentRules};
