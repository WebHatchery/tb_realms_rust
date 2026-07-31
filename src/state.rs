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

pub mod honours;
pub mod licences;
pub mod portfolio;
pub mod settlement;

pub use licences::{LicenceError, LicenceTable, Licences};
pub use portfolio::{OrderError, Portfolio, PortfolioSave};
pub use settlement::SettlementRecord;

use settlement::movers;

use crate::data::{GameConfig, GameData};
use crate::market::orders::{Fill, OrderConfig, Side};
use crate::market::{Board, BoardChange, Instrument, MarketConfig};
use crate::world::delta::WorldDelta;
use crate::world::fundamentals::EntityRef;
use crate::world::Snapshot;
use macroquad_toolkit::achievements::Achievements;
use macroquad_toolkit::rng::SeededRng;
use macroquad_toolkit::series::Series;
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
    /// How far up the information ladder the broker has bought. Progression,
    /// and the other thing no world view can reconstruct.
    #[serde(default)]
    pub licences: Licences,
    /// What the broker has been noticed doing.
    #[serde(default)]
    pub honours: Achievements,
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
    /// How far the broker has climbed the information ladder (GDD 5.3).
    licences: Licences,
    /// What the broker has been noticed doing. No reward but the record: a
    /// bonus here would compete with the licences, which are meant to be the
    /// only progression axis (Pillar 3).
    honours: Achievements,
    /// How long each position has been held, in settlements, for the honours
    /// that are about patience rather than profit.
    held_for: BTreeMap<EntityRef, u32>,
    /// The era the world was in when last read, so a turning of the age can be
    /// noticed rather than merely happening.
    era: u32,
    /// The broker's equity at each settlement — the curve the records screen
    /// draws, and the one number that says whether any of this worked.
    equity_curve: Series,
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

/// Settlements a position must be held through, unbroken, to count as patience.
const PATIENT_SETTLEMENTS: u32 = 10;

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
    /// The equity curve, the licences, and the seeds behind the century.
    Records,
}

/// Everything one reading of the world did.
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub board: BoardChange,
    pub books: SettlementBooks,
    /// An age ended between this reading and the last. The largest event in the
    /// game, and the one nothing could have priced (GDD 5.6).
    pub cataclysm: bool,
    /// Honours newly won by this settlement, by id.
    pub won: Vec<String>,
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
            licences: Licences::default(),
            honours: Achievements::new(),
            held_for: BTreeMap::new(),
            era: 0,
            equity_curve: Series::new(market.history_capacity),
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

    /// Move the selection along the board by `rows`, for a keyboard.
    ///
    /// Walks the same order the board is drawn in — dearest first, and only the
    /// paper this licence reaches — so what the arrows do matches what the eye
    /// sees. Nothing selected yet lands on the top of the board.
    pub fn step_selection(&mut self, rows: i32, table: &LicenceTable) {
        let held = self.licences.held;
        let visible: Vec<&Instrument> = {
            let mut listings: Vec<&Instrument> = self
                .board
                .listings()
                .filter(|listing| {
                    listing
                        .entity
                        .listing_class()
                        .is_some_and(|class| table.permits(held, class))
                })
                .collect();
            listings.sort_by(|a, b| b.fair_value.total_cmp(&a.fair_value));
            listings
        };
        if visible.is_empty() {
            return;
        }
        let here = self
            .selected
            .as_ref()
            .and_then(|entity| visible.iter().position(|l| &l.entity == entity));
        let next = match here {
            Some(index) => (index as i32 + rows).clamp(0, visible.len() as i32 - 1) as usize,
            None => 0,
        };
        self.selected = Some(visible[next].entity.clone());
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
        let realised_before = self.portfolio.realised;
        let fill = self.portfolio.execute(
            &listing,
            side,
            self.ticket_units,
            self.settlements.first().map(|s| s.year).unwrap_or(0),
            orders,
        )?;
        *self.order_flow.entry(entity).or_default() += fill.flow;
        honours::review_fill(
            &mut self.honours,
            &fill,
            &self.board,
            realised_before,
            self.portfolio.realised,
        );
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
        era: u32,
        data: &GameData,
    ) -> Observed {
        let market = &data.config.market;
        let board = self
            .board
            .refresh(&snapshot, &data.instruments, market, era);

        // An age ending is the largest, least-priceable event in the game
        // (GDD 5.6). The exchange cannot see it coming; it can only notice that
        // it has happened, and it must never do so twice.
        let cataclysm = self.era != 0 && era != self.era;
        self.era = era;

        let mut books = SettlementBooks::default();
        let mut won = Vec::new();
        if let Some(delta) = turning {
            let shocks = data.impacts.shocks(delta, &snapshot);
            self.board
                .settle(&shocks, &self.order_flow, market, era, &mut self.market_rng);

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
            let mut record = SettlementRecord::from_delta(delta, &board, movers(&self.board));
            record.cataclysm = cataclysm;
            self.settlements.insert(0, record);
            self.settlements.truncate(LOG_CAPACITY);

            // The chronicle is kept whole and separate from the settlement log:
            // it is the newspaper, and a reader wants a run of it rather than
            // one line per year (GDD 5.5).
            for event in delta.events.iter().rev() {
                self.chronicle.insert(0, event.clone());
            }
            self.chronicle.truncate(CHRONICLE_CAPACITY);

            // One point per settlement, so the curve's x-axis is the world's
            // clock and not the wall's.
            self.equity_curve.push(self.portfolio.equity(&self.board));

            self.age_positions();
            let standing = honours::Standing {
                portfolio: &self.portfolio,
                board: &self.board,
                licences: self.licences.held,
                observed: self.observed,
                cataclysm,
                written_off: books.written_off.len(),
                income: books.income,
            };
            won = honours::review(&mut self.honours, &standing);
        }

        self.snapshot = snapshot;
        Observed {
            board,
            books,
            cataclysm,
            won,
        }
    }

    /// Count another settlement against every position still held, and forget
    /// the ones that are not.
    ///
    /// Patience is measured against a *continuously* held position: selling out
    /// and buying back in starts the count again, which is the difference
    /// between holding through a bad decade and merely owning something twice.
    fn age_positions(&mut self) {
        self.held_for
            .retain(|entity, _| self.portfolio.units_of(entity) > 0);
        for position in self.portfolio.positions() {
            let years = self.held_for.entry(position.entity.clone()).or_insert(0);
            *years += 1;
            if *years >= PATIENT_SETTLEMENTS {
                self.honours.unlock("patient_money");
            }
        }
    }

    pub fn honours(&self) -> &Achievements {
        &self.honours
    }

    /// Bring the honours in line with the shipped definitions.
    ///
    /// Called after construction and after a load: `sync_definitions` keeps what
    /// has been won while picking up renamed text and achievements added since
    /// the save was written, which is the whole reason the toolkit has it.
    pub fn adopt_honours(
        &mut self,
        definitions: Vec<macroquad_toolkit::achievements::Achievement>,
    ) {
        self.honours.sync_definitions(definitions);
    }

    /// The broker's equity at every settlement observed.
    pub fn equity_curve(&self) -> &Series {
        &self.equity_curve
    }

    pub fn licences(&self) -> Licences {
        self.licences
    }

    /// Whether this broker may trade a class at all (GDD 5.3).
    pub fn licensed_for(&self, class: &str, table: &LicenceTable) -> bool {
        table.permits(self.licences.held, class)
    }

    /// Buy the next rung of the ladder.
    ///
    /// Paid for out of the same cash that buys paper, which is the whole
    /// tension of Pillar 3: every crown spent on seeing further is a crown not
    /// invested, and a broker who buys sight too early starves.
    pub fn buy_licence(&mut self, table: &LicenceTable) -> Result<String, LicenceError> {
        let Some(next) = table.next(self.licences.held) else {
            return Err(LicenceError::Complete);
        };
        if next.cost > self.portfolio.cash {
            return Err(LicenceError::Insufficient {
                need: next.cost,
                have: self.portfolio.cash,
            });
        }
        self.portfolio.cash -= next.cost;
        self.licences.held += 1;
        Ok(next.name.clone())
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
            licences: self.licences,
            honours: self.honours.clone(),
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
            licences: save.licences,
            honours: save.honours.clone(),
            held_for: BTreeMap::new(),
            era: 0,
            equity_curve: Series::new(market.history_capacity),
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
        licences: Licences::default(),
        honours: Achievements::new(),
    })
}

#[cfg(test)]
mod tests;
