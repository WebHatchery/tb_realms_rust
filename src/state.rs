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

pub use portfolio::{OrderError, Portfolio, PortfolioSave};

use crate::data::{GameConfig, GameData};
use crate::market::orders::{Fill, OrderConfig, Side};
use crate::market::{Board, BoardChange, Instrument, MarketConfig};
use crate::world::delta::{Change, WorldDelta};
use crate::world::fundamentals::EntityRef;
use crate::world::Snapshot;
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// How many settlements the in-memory log keeps. The exchange runs for
/// centuries, so this is a display buffer, not the archive; the archive is the
/// records screen's job once there are trades to record (GDD 9).
const LOG_CAPACITY: usize = 64;

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
}

/// How many appearances/disappearances a single record names before it stops.
const NOTABLE_LIMIT: usize = 3;

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
    fn from_delta(delta: &WorldDelta, board: &BoardChange) -> Self {
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
        }
    }
}

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
        }

        self.snapshot = snapshot;
        Observed { board, books }
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

    /// The market noise stream's whole state — its seed and everything drawn
    /// from it since. Shown on the records screen, and saved.
    pub fn market_state(&self) -> u64 {
        self.market_rng.state()
    }

    /// Record a turning of the world, and what it did to the board.
    pub fn record(&mut self, delta: &WorldDelta, board: &BoardChange) {
        self.last_tick = delta.to_tick;
        self.observed += 1;
        self.settlements
            .insert(0, SettlementRecord::from_delta(delta, board));
        self.settlements.truncate(LOG_CAPACITY);
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
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::delta::WorldDelta;
    use mytherra_protocol::{project_world, spectator_standing, WorldView};

    fn data() -> GameData {
        GameData::load().expect("the exchange's data loads")
    }

    fn market() -> MarketConfig {
        data().config.market
    }

    fn orders() -> OrderConfig {
        data().config.orders
    }

    /// A freshly seeded world as a spectator receives it.
    fn world() -> WorldView {
        let content = mytherra_core::data::GameData::load().expect("mytherra content loads");
        project_world(
            &mytherra_core::world::WorldState::new(&content),
            &spectator_standing(),
        )
    }

    fn delta(year: u32, tick: u64) -> WorldDelta {
        WorldDelta {
            to_tick: tick,
            from_year: year.saturating_sub(1),
            to_year: year,
            ..Default::default()
        }
    }

    #[test]
    fn settlements_are_logged_newest_first_and_bounded() {
        let mut session = GameSession::new(&market(), &orders());
        for year in 1..=(LOG_CAPACITY as u32 + 10) {
            session.record(&delta(year, year as u64), &BoardChange::default());
        }

        assert_eq!(
            session.settlements().len(),
            LOG_CAPACITY,
            "the log is bounded"
        );
        assert_eq!(
            session.settlements()[0].year,
            LOG_CAPACITY as u32 + 10,
            "newest first"
        );
        // The count outlives the buffer, so the digest can say how long the
        // exchange has been open even after the log has rolled.
        assert_eq!(session.observed, LOG_CAPACITY as u64 + 10);
        assert_eq!(session.last_tick, LOG_CAPACITY as u64 + 10);
    }

    #[test]
    fn a_save_round_trips_the_cursor_and_tick() {
        let mut session = GameSession::new(&market(), &orders());
        session.record(&delta(341, 17), &BoardChange::default());

        let save = session.to_save("0.1.0", 4_212);
        let resumed = GameSession::from_save(&save, &market(), &orders());

        assert_eq!(resumed.last_tick, 17);
        // Neither the log nor the board is persisted — the log is a display
        // buffer and the board is a function of the world, and the next read
        // repopulates both.
        assert!(resumed.settlements().is_empty());
        assert!(resumed.board().is_empty());
    }

    #[test]
    fn the_board_hangs_from_the_world_and_the_digest_names_the_tickers() {
        let data = data();
        let view = world();

        let mut session = GameSession::new(&market(), &orders());
        let opening = session.observe(Snapshot::read(&view), None, &data).board;

        assert!(!session.board().is_empty(), "the world hangs the board");
        assert_eq!(opening.listed.len(), session.board().len());
        assert!(opening.delisted.is_empty());
        // The reading is kept, so the listing screen can show the fundamentals
        // a price was derived from without asking the world again.
        assert!(session.snapshot().iter().count() > 0);

        // A settlement's digest reports the board's own tickers, so a razed
        // town reads as paper struck off rather than as an entity count.
        let mut razed = view.clone();
        let lost = razed.settlements.remove(0);
        let change = session.observe(Snapshot::read(&razed), None, &data).board;
        session.record(&delta(razed.year + 1, 2), &change);

        assert_eq!(change.delisted.len(), 1);
        assert_eq!(session.settlements()[0].delisted, 1);
        assert!(
            session.settlements()[0].notable[0].ends_with("delisted"),
            "{:?}",
            session.settlements()[0].notable
        );
        assert!(
            session
                .board()
                .listings()
                .all(|listing| listing.name != lost.name),
            "the razed town is off the board"
        );
    }

    #[test]
    fn an_unreadable_save_starts_fresh_rather_than_failing() {
        // The shipped config with only the version bumped, so a field added to
        // it later cannot make this test lie about what migration does.
        let config = GameConfig {
            version: "0.2.0".to_owned(),
            ..data().config
        };
        let value = serde_json::json!({ "nonsense": true });

        let migrated = migrate_save_value(Some("0.0.1".to_owned()), value, &config).unwrap();

        assert_eq!(migrated.version, "0.2.0");
        assert_eq!(migrated.cursor, 0);
        assert_eq!(migrated.last_tick, 0);
    }

    #[test]
    fn a_current_save_survives_migration_with_its_cursor_intact() {
        // The shipped config with only the version bumped, so a field added to
        // it later cannot make this test lie about what migration does.
        let config = GameConfig {
            version: "0.2.0".to_owned(),
            ..data().config
        };
        let value = serde_json::json!({
            "version": "0.1.0",
            "cursor": 918,
            "last_tick": 16_290
        });

        let migrated = migrate_save_value(Some("0.1.0".to_owned()), value, &config).unwrap();

        assert_eq!(migrated.version, "0.2.0", "stamped to the current version");
        assert_eq!(migrated.cursor, 918, "the cursor is not lost");
        assert_eq!(migrated.last_tick, 16_290);
    }
}
