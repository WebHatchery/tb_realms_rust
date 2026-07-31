//! What the broker owns, what it cost, and what it is worth (GDD 5, 7).
//!
//! This is the first state in the game that is genuinely *not* re-derivable
//! from the world. The board can be rebuilt from any `WorldView`; a position
//! cannot be rebuilt from anything, so this is what a save is actually for.
//!
//! Two rules give the ledger its shape:
//!
//! - **No short selling and no borrowing.** A broker may not sell what they do
//!   not hold or buy what they cannot pay for. Leverage is a licence unlocked
//!   later (GDD 4); until then the ledger cannot go negative in either
//!   direction, which makes every P&L figure below unambiguous.
//! - **A delisted position is a total loss.** When a house's blood runs out or
//!   a town is razed, the paper is not marked down — it is worthless, and the
//!   loss is realised at nothing. That is the risk that makes GDD 8's emergent
//!   delistings matter rather than being trivia.
//!
//! No macroquad here either: the backtest harness trades through this.

use crate::market::orders::{quote, Fill, OrderConfig, Side};
use crate::market::{Board, Instrument};
use crate::world::fundamentals::EntityRef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How many fills the in-memory ledger keeps for display. The records archive
/// is a later screen's job (GDD 9); this is the tape on the desk.
const LEDGER_CAPACITY: usize = 128;

/// Why an order was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum OrderError {
    /// The size was zero or negative.
    NoUnits,
    /// Selling more than is held. There is no short selling.
    Short { held: i64 },
    /// Buying more than the ledger can pay for.
    Insufficient { need: f32, have: f32 },
}

impl OrderError {
    pub fn label(&self) -> String {
        match self {
            OrderError::NoUnits => "an order of nothing is not an order".to_owned(),
            OrderError::Short { held } => {
                format!("you hold {held} — the exchange does not lend paper")
            }
            OrderError::Insufficient { need, have } => {
                format!("that costs {need:.0}; the ledger holds {have:.0}")
            }
        }
    }
}

/// A holding in one listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub entity: EntityRef,
    /// The ticker as it was when the position opened, so the ledger reads the
    /// same after the world renames something.
    pub symbol: String,
    pub units: i64,
    /// Everything paid for the units still held, fees included. Fees are folded
    /// in on purpose: a P&L that ignores what the trading cost is a P&L that
    /// flatters the trader.
    pub cost_basis: f32,
}

impl Position {
    pub fn average_cost(&self) -> f32 {
        if self.units == 0 {
            return 0.0;
        }
        self.cost_basis / self.units as f32
    }

    pub fn value(&self, price: f32) -> f32 {
        price * self.units as f32
    }

    pub fn unrealised(&self, price: f32) -> f32 {
        self.value(price) - self.cost_basis
    }
}

/// The broker's ledger.
#[derive(Debug, Clone)]
pub struct Portfolio {
    pub cash: f32,
    positions: BTreeMap<EntityRef, Position>,
    /// Profit and loss on positions already closed, or delisted out from under
    /// the holder.
    pub realised: f32,
    pub fees_paid: f32,
    pub income_received: f32,
    /// The capital the ledger opened with, so the records screen can say how
    /// the century actually went.
    pub opening_capital: f32,
    ledger: Vec<Fill>,
}

/// The portfolio as it is saved. Split from the live struct so the bounded
/// display tape is not persisted — it is a view, not a fact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortfolioSave {
    pub cash: f32,
    pub positions: Vec<Position>,
    pub realised: f32,
    pub fees_paid: f32,
    pub income_received: f32,
    pub opening_capital: f32,
}

impl Portfolio {
    pub fn new(config: &OrderConfig) -> Self {
        Self {
            cash: config.starting_capital,
            positions: BTreeMap::new(),
            realised: 0.0,
            fees_paid: 0.0,
            income_received: 0.0,
            opening_capital: config.starting_capital,
            ledger: Vec::new(),
        }
    }

    /// Price an order exactly as executing it would. The ticket shows this, so
    /// what a player is quoted is what they get.
    pub fn preview(
        &self,
        listing: &Instrument,
        side: Side,
        units: i64,
        year: u32,
        config: &OrderConfig,
    ) -> Fill {
        quote(listing, side, units, year, config)
    }

    /// Whether an order would be accepted, without executing it.
    pub fn check(&self, fill: &Fill) -> Result<(), OrderError> {
        if fill.units <= 0 {
            return Err(OrderError::NoUnits);
        }
        match fill.side {
            Side::Buy if -fill.cash > self.cash => Err(OrderError::Insufficient {
                need: -fill.cash,
                have: self.cash,
            }),
            Side::Sell => {
                let held = self.units_of(&fill.entity);
                if fill.units > held {
                    return Err(OrderError::Short { held });
                }
                Ok(())
            }
            Side::Buy => Ok(()),
        }
    }

    /// Execute an order, or explain why not.
    pub fn execute(
        &mut self,
        listing: &Instrument,
        side: Side,
        units: i64,
        year: u32,
        config: &OrderConfig,
    ) -> Result<Fill, OrderError> {
        let fill = self.preview(listing, side, units, year, config);
        self.check(&fill)?;

        self.cash += fill.cash;
        self.fees_paid += fill.fee;

        match side {
            Side::Buy => {
                let position = self
                    .positions
                    .entry(fill.entity.clone())
                    .or_insert_with(|| Position {
                        entity: fill.entity.clone(),
                        symbol: fill.symbol.clone(),
                        units: 0,
                        cost_basis: 0.0,
                    });
                position.units += fill.units;
                // The fee joins the basis, so what the position "cost" is what
                // leaving the ledger actually cost.
                position.cost_basis += fill.gross + fill.fee;
            }
            Side::Sell => {
                // Average cost, not lots: the basis leaves in proportion to the
                // units, so the realised figure does not depend on which
                // imaginary certificate was handed over.
                let sold_basis = {
                    let position = self
                        .positions
                        .get_mut(&fill.entity)
                        .expect("checked above that the units are held");
                    let share = fill.units as f32 / position.units as f32;
                    let basis = position.cost_basis * share;
                    position.units -= fill.units;
                    position.cost_basis -= basis;
                    basis
                };
                self.realised += fill.gross - fill.fee - sold_basis;
                self.drop_if_empty(&fill.entity);
            }
        }

        self.ledger.insert(0, fill.clone());
        self.ledger.truncate(LEDGER_CAPACITY);
        Ok(fill)
    }

    /// Pay every position's coupon for one settlement (GDD 5.2).
    pub fn collect_income(&mut self, board: &Board) -> f32 {
        let mut paid = 0.0;
        for position in self.positions.values() {
            if let Some(listing) = board.get(&position.entity) {
                paid += listing.income * position.units as f32;
            }
        }
        self.cash += paid;
        self.income_received += paid;
        paid
    }

    /// Write off every position whose listing has left the board.
    ///
    /// The claim is not marked down, it is gone: the house's blood ran out, the
    /// town was razed. The whole basis is realised as a loss, and the symbols
    /// are returned so the digest can say which paper died.
    pub fn write_off_delisted(&mut self, board: &Board) -> Vec<String> {
        let dead: Vec<EntityRef> = self
            .positions
            .keys()
            .filter(|entity| board.get(entity).is_none())
            .cloned()
            .collect();

        let mut struck = Vec::new();
        for entity in dead {
            if let Some(position) = self.positions.remove(&entity) {
                self.realised -= position.cost_basis;
                struck.push(position.symbol);
            }
        }
        struck
    }

    /// Cash plus everything held, at the board's current prices.
    pub fn equity(&self, board: &Board) -> f32 {
        self.cash + self.holdings_value(board)
    }

    /// What the positions are worth, ignoring cash.
    pub fn holdings_value(&self, board: &Board) -> f32 {
        self.positions
            .values()
            .filter_map(|position| {
                board
                    .get(&position.entity)
                    .map(|listing| position.value(listing.price))
            })
            .sum()
    }

    /// Profit and loss on what is still held.
    pub fn unrealised(&self, board: &Board) -> f32 {
        self.positions
            .values()
            .filter_map(|position| {
                board
                    .get(&position.entity)
                    .map(|listing| position.unrealised(listing.price))
            })
            .sum()
    }

    pub fn units_of(&self, entity: &EntityRef) -> i64 {
        self.positions
            .get(entity)
            .map(|position| position.units)
            .unwrap_or(0)
    }

    pub fn position(&self, entity: &EntityRef) -> Option<&Position> {
        self.positions.get(entity)
    }

    /// Every holding, in a stable order.
    pub fn positions(&self) -> impl Iterator<Item = &Position> {
        self.positions.values()
    }

    /// The tape, newest first.
    pub fn ledger(&self) -> &[Fill] {
        &self.ledger
    }

    pub fn to_save(&self) -> PortfolioSave {
        PortfolioSave {
            cash: self.cash,
            positions: self.positions.values().cloned().collect(),
            realised: self.realised,
            fees_paid: self.fees_paid,
            income_received: self.income_received,
            opening_capital: self.opening_capital,
        }
    }

    pub fn from_save(save: &PortfolioSave, config: &OrderConfig) -> Self {
        // An empty save is one written before there were positions; open a
        // fresh ledger rather than a bankrupt one.
        if save.opening_capital <= 0.0 {
            return Self::new(config);
        }
        Self {
            cash: save.cash,
            positions: save
                .positions
                .iter()
                .map(|position| (position.entity.clone(), position.clone()))
                .collect(),
            realised: save.realised,
            fees_paid: save.fees_paid,
            income_received: save.income_received,
            opening_capital: save.opening_capital,
            ledger: Vec::new(),
        }
    }

    fn drop_if_empty(&mut self, entity: &EntityRef) {
        if self.units_of(entity) == 0 {
            self.positions.remove(entity);
        }
    }
}

#[cfg(test)]
mod tests;
