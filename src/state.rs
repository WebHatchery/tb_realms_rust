//! The exchange's own state: what it has watched the world do, what it
//! currently lists, what the broker holds, and what of that survives a restart.
//!
//! Three things live here and they are saved on very different terms:
//!
//! - The [`Board`] is **not** saved. It is a function of the world, and the next
//!   read rebuilds it exactly; persisting it would only create a way for the
//!   exchange's idea of what exists to drift from the world's.
//! - The settlement log is **not** saved. It is a display buffer.
//! - The [`Portfolio`] **is** saved, because it is the one thing here that no
//!   world view can reconstruct.

pub mod portfolio;
pub mod settlement;

pub use portfolio::{OrderError, Portfolio, PortfolioSave};
pub use settlement::SettlementRecord;

use settlement::movers;

use crate::data::{GameConfig, GameData};
use crate::market::orders::{Fill, OrderConfig, Side};
use crate::market::{Board, BoardChange, Instrument, MarketConfig};
use crate::world::delta::WorldDelta;
use crate::world::fundamentals::EntityRef;
use crate::world::Snapshot;
use macroquad_toolkit::rng::SeededRng;
use mytherra_core::world::{EventKind, WorldEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// How many settlements the in-memory log keeps. The exchange runs for
/// centuries, so this is a display buffer, not the archive; the archive is the
/// records screen's job once there are trades to record (GDD 9).
const LOG_CAPACITY: usize = 64;

/// How many chronicle lines the exchange keeps to read back. The world's own
/// archive is far longer; this is the run of newspapers on the desk.
const CHRONICLE_CAPACITY: usize = 400;

/// The class the research browser opens on. Provinces: the broadest paper, and
/// the one whose fundamentals a newcomer can read without knowing anything.
const DEFAULT_RESEARCH_CLASS: &str = "regional_bond";

/// What survives a restart. Deliberately small: everything else in the session
/// is re-derived from the world on the next read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    /// The chronicle cursor last acknowledged.
    pub cursor: u64,
    /// The world tick the exchange had last seen.
    pub last_tick: u64,
    /// The market noise stream's state, so a reloaded exchange draws the same
    /// numbers it would have drawn had it never closed (GDD 5.8). Defaulted so
    /// a save written before prices existed still loads.
    #[serde(default)]
    pub market_state: u64,
    /// The ledger. The only part of this save that could not be rebuilt from
    /// the world on the next read.
    #[serde(default)]
    pub portfolio: PortfolioSave,
}

/// The live session.
#[derive(Debug, Clone)]
pub struct GameSession {
    /// Settlements observed, newest first.
    settlements: Vec<SettlementRecord>,
    /// What the exchange currently lists — the world's own answer, not the
    /// exchange's opinion.
    board: Board,
    /// The broker's ledger — the one thing here a world view cannot rebuild.
    portfolio: Portfolio,
    /// The mark the player's own orders have left on each price, decaying each
    /// settlement. This is GDD 5.4's `flow` term: your size is not free, and it
    /// is still costing you a year after you traded.
    order_flow: BTreeMap<EntityRef, f32>,
    /// The world as it was last read, kept so the research and listing screens
    /// can show the fundamentals a price is derived from without re-reading it.
    /// It is a view of the world, never a second opinion about it.
    snapshot: Snapshot,
    /// Which screen the exchange is showing.
    screen: Screen,
    /// The chronicle as read so far, newest first.
    chronicle: Vec<WorldEvent>,
    /// Which kind of chronicle line the reader is filtering to, if any.
    chronicle_filter: Option<EventKind>,
    /// The instrument class the research browser is showing.
    research_class: String,
    /// The listing the desk is looking at, if any.
    selected: Option<EntityRef>,
    /// The size on the ticket.
    pub ticket_units: i64,
    /// The market's own noise stream, kept apart from anything the world uses
    /// so the two cannot desynchronise each other (GDD 5.8). Its state is saved,
    /// so a reloaded exchange continues the same century rather than starting a
    /// new one.
    market_rng: SeededRng,
    /// The world tick last seen, or zero before the first world arrives.
    pub last_tick: u64,
    /// Total settlements observed this session, which outlives the bounded log.
    pub observed: u64,
}

/// The size a ticket opens at, and the step its buttons move by.
const TICKET_STEP: i64 = 10;

/// Which screen the exchange is showing (GDD 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The board, the ledger, and the desk.
    Floor,
    /// One listing in full: its chart, its fundamentals, and why it moved.
    Listing,
    /// The world in its own words, beside what actually moved.
    Chronicle,
    /// Every entity the exchange watches, class by class.
    Research,
}

/// Everything one reading of the world did.
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub board: BoardChange,
    pub books: SettlementBooks,
}

/// What a settlement did to the ledger, as distinct from what it did to prices.
#[derive(Debug, Clone, Default)]
pub struct SettlementBooks {
    /// Coupons paid by everything held.
    pub income: f32,
    /// Positions written off because the world stopped carrying the thing they
    /// were a claim on. A total loss, by design.
    pub written_off: Vec<String>,
}

impl GameSession {
    pub fn new(market: &MarketConfig, orders: &OrderConfig) -> Self {
        Self {
            settlements: Vec::new(),
            board: Board::new(),
            portfolio: Portfolio::new(orders),
            order_flow: BTreeMap::new(),
            snapshot: Snapshot::default(),
            screen: Screen::Floor,
            chronicle: Vec::new(),
            chronicle_filter: None,
            research_class: DEFAULT_RESEARCH_CLASS.to_owned(),
            selected: None,
            ticket_units: TICKET_STEP,
            market_rng: SeededRng::new(market.seed),
            last_tick: 0,
            observed: 0,
        }
    }

    pub fn portfolio(&self) -> &Portfolio {
        &self.portfolio
    }

    /// The listing the desk is looking at, if it is still listed. A selection
    /// whose paper was struck off simply reads as no selection.
    pub fn selected(&self) -> Option<&Instrument> {
        self.board.get(self.selected.as_ref()?)
    }

    pub fn select(&mut self, entity: EntityRef) {
        self.selected = Some(entity);
    }

    /// Resize the ticket, never below one unit.
    pub fn resize_ticket(&mut self, by: i64) {
        self.ticket_units = (self.ticket_units + by).max(1);
    }

    /// Place an order against the selected listing.
    ///
    /// The flow it leaves is accumulated here rather than applied now: an
    /// order's mark on the price arrives at the *next* settlement, which is what
    /// makes it a cost the trader cannot immediately trade back out of.
    pub fn order(&mut self, side: Side, orders: &OrderConfig) -> Result<Fill, OrderError> {
        let Some(entity) = self.selected.clone() else {
            return Err(OrderError::NoUnits);
        };
        let Some(listing) = self.board.get(&entity).cloned() else {
            return Err(OrderError::NoUnits);
        };
        let fill = self.portfolio.execute(
            &listing,
            side,
            self.ticket_units,
            self.settlements.first().map(|s| s.year).unwrap_or(0),
            orders,
        )?;
        *self.order_flow.entry(entity).or_default() += fill.flow;
        Ok(fill)
    }

    /// What the selected listing would cost at the ticket's size, and whether
    /// the ledger would allow it. Shown before trading, and computed by exactly
    /// the code that would execute it.
    pub fn preview(
        &self,
        side: Side,
        orders: &OrderConfig,
    ) -> Option<(Fill, Result<(), OrderError>)> {
        let listing = self.selected()?;
        let fill = self.portfolio.preview(
            listing,
            side,
            self.ticket_units,
            self.settlements.first().map(|s| s.year).unwrap_or(0),
            orders,
        );
        let allowed = self.portfolio.check(&fill);
        Some((fill, allowed))
    }

    /// Take in a fresh reading of the world.
    ///
    /// One call rather than two on purpose: the board must be revalued *before*
    /// prices settle, or every price would drift toward a fair value a year
    /// stale. Keeping both steps behind one function means that ordering cannot
    /// be got wrong by a caller.
    ///
    /// `turning` is `None` on the first view to arrive: the board hangs, but
    /// nothing settles, because no year has passed for the exchange to price.
    pub fn observe(
        &mut self,
        snapshot: Snapshot,
        turning: Option<&WorldDelta>,
        data: &GameData,
    ) -> Observed {
        let market = &data.config.market;
        let board = self.board.refresh(&snapshot, &data.instruments, market);

        let mut books = SettlementBooks::default();
        if let Some(delta) = turning {
            let shocks = data.impacts.shocks(delta, &snapshot);
            self.board
                .settle(&shocks, &self.order_flow, market, &mut self.market_rng);

            // The floor forgets a large order gradually. Decaying after settling
            // rather than before means the year in which an order was struck is
            // the year it hits hardest, which is the honest ordering.
            for flow in self.order_flow.values_mut() {
                *flow *= data.config.orders.flow_decay;
            }
            self.order_flow.retain(|_, flow| flow.abs() > 0.0001);

            books = SettlementBooks {
                income: self.portfolio.collect_income(&self.board),
                written_off: self.portfolio.write_off_delisted(&self.board),
            };

            // Logged here rather than by the caller because a settlement's
            // record includes what the *settled* board did, which only exists
            // after the two steps above have run.
            self.last_tick = delta.to_tick;
            self.observed += 1;
            self.settlements.insert(
                0,
                SettlementRecord::from_delta(delta, &board, movers(&self.board)),
            );
            self.settlements.truncate(LOG_CAPACITY);

            // The chronicle is kept whole and separate from the settlement log:
            // it is the newspaper, and a reader wants a run of it rather than
            // one line per year (GDD 5.5).
            for event in delta.events.iter().rev() {
                self.chronicle.insert(0, event.clone());
            }
            self.chronicle.truncate(CHRONICLE_CAPACITY);
        }

        self.snapshot = snapshot;
        Observed { board, books }
    }

    /// The chronicle as read so far, newest first. Prose, and nothing that
    /// prices may read it (GDD 5.5).
    pub fn chronicle(&self) -> &[WorldEvent] {
        &self.chronicle
    }

    /// Fill the chronicle from the world's own archive, once.
    ///
    /// The first view carries the recent newspaper already, so an exchange that
    /// has just opened should show it rather than a blank page until the world
    /// next turns — the world has a past whether or not this session watched it
    /// happen. Only ever fills an empty chronicle: after that, the lines arrive
    /// a year at a time and the archive would duplicate them.
    pub fn seed_chronicle(&mut self, archive: &[WorldEvent]) {
        if !self.chronicle.is_empty() || archive.is_empty() {
            return;
        }
        self.chronicle = archive.iter().take(CHRONICLE_CAPACITY).cloned().collect();
    }

    /// The world as last read, for the screens that show fundamentals.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    /// Open a listing's own page, which only makes sense for paper that exists.
    pub fn open_listing(&mut self, entity: EntityRef) {
        if self.board.get(&entity).is_some() {
            self.selected = Some(entity);
            self.screen = Screen::Listing;
        }
    }

    pub fn close_listing(&mut self) {
        self.screen = Screen::Floor;
    }

    /// Show a screen. A listing's page is reached through `open_listing`
    /// instead, because it needs to know *which* listing.
    pub fn show(&mut self, screen: Screen) {
        self.screen = screen;
    }

    pub fn chronicle_filter(&self) -> Option<EventKind> {
        self.chronicle_filter
    }

    pub fn filter_chronicle(&mut self, kind: Option<EventKind>) {
        self.chronicle_filter = kind;
    }

    pub fn research_class(&self) -> &str {
        &self.research_class
    }

    pub fn research(&mut self, class: String) {
        self.research_class = class;
    }

    /// The market noise stream's whole state — its seed and everything drawn
    /// from it since. Shown on the records screen, and saved.
    pub fn market_state(&self) -> u64 {
        self.market_rng.state()
    }

    /// Settlements observed, newest first.
    pub fn settlements(&self) -> &[SettlementRecord] {
        &self.settlements
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn to_save(&self, version: &str, cursor: u64) -> SaveData {
        SaveData {
            version: version.to_owned(),
            cursor,
            last_tick: self.last_tick,
            market_state: self.market_rng.state(),
            portfolio: self.portfolio.to_save(),
        }
    }

    pub fn from_save(save: &SaveData, market: &MarketConfig, orders: &OrderConfig) -> Self {
        Self {
            settlements: Vec::new(),
            // The board is a function of the world, so it is rebuilt from the
            // next read rather than restored from a save that could disagree
            // with what the world now carries.
            board: Board::new(),
            portfolio: Portfolio::from_save(&save.portfolio, orders),
            snapshot: Snapshot::default(),
            screen: Screen::Floor,
            // The chronicle is re-read from the world's own archive on the next
            // poll, from the cursor this save carries.
            chronicle: Vec::new(),
            chronicle_filter: None,
            research_class: DEFAULT_RESEARCH_CLASS.to_owned(),
            // Flow is not saved: it decays to nothing within a few settlements
            // anyway, and a mark left on a price by an order struck before a
            // restart is not worth a migration to carry.
            order_flow: BTreeMap::new(),
            selected: None,
            ticket_units: TICKET_STEP,
            // A state of zero means a save written before prices existed; start
            // that session's stream from the seed rather than from nothing.
            market_rng: if save.market_state == 0 {
                SeededRng::new(market.seed)
            } else {
                SeededRng::from_state(save.market_state)
            },
            last_tick: save.last_tick,
            observed: 0,
        }
    }
}

/// Bring a save of any earlier shape up to the current one.
///
/// The exchange's save is small and re-derivable, so an unreadable one is not
/// worth failing over: an unrecognised payload starts a fresh session rather
/// than refusing to open, and the next read of the world repopulates it.
pub fn migrate_save_value(
    _detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);

    if let Ok(mut current) = serde_json::from_value::<SaveData>(payload) {
        current.version = config.version.clone();
        return Ok(current);
    }

    Ok(SaveData {
        version: config.version.clone(),
        cursor: 0,
        last_tick: 0,
        market_state: 0,
        portfolio: PortfolioSave::default(),
    })
}

#[cfg(test)]
mod tests;
