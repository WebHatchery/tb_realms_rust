//! The exchange's own state: what it has watched the world do, what it
//! currently lists, and what of that survives a restart.
//!
//! There are no positions here yet — prices and orders arrive in M2 (GDD 13).
//! What exists now is the [`Board`], derived wholly from the world, and the
//! settlement log: one record per turning, which is what the digest and records
//! screens are built from. The chronicle cursor rides along so a reopened
//! exchange resumes reading where it stopped rather than re-reading a century of
//! prose.
//!
//! The board is deliberately **not** saved. It is a function of the world, and
//! the next read rebuilds it exactly; persisting it would only create a way for
//! the exchange's idea of what exists to drift from the world's.

use crate::data::GameConfig;
use crate::market::{Board, BoardChange, InstrumentRules};
use crate::world::delta::{Change, WorldDelta};
use crate::world::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

/// The live session.
#[derive(Debug, Clone, Default)]
pub struct GameSession {
    /// Settlements observed, newest first.
    settlements: Vec<SettlementRecord>,
    /// What the exchange currently lists — the world's own answer, not the
    /// exchange's opinion.
    board: Board,
    /// The world tick last seen, or zero before the first world arrives.
    pub last_tick: u64,
    /// Total settlements observed this session, which outlives the bounded log.
    pub observed: u64,
}

impl GameSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bring the board in line with a fresh reading of the world.
    ///
    /// Separate from [`record`](Self::record) because the two answer different
    /// questions: the board must hang the moment the world first answers, while
    /// a settlement is only logged when the world actually turns.
    pub fn list(&mut self, snapshot: &Snapshot, rules: &InstrumentRules) -> BoardChange {
        self.board.refresh(snapshot, rules)
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
        }
    }

    pub fn from_save(save: &SaveData) -> Self {
        Self {
            settlements: Vec::new(),
            // The board is a function of the world, so it is rebuilt from the
            // next read rather than restored from a save that could disagree
            // with what the world now carries.
            board: Board::new(),
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::delta::WorldDelta;

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
        let mut session = GameSession::new();
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
        let mut session = GameSession::new();
        session.record(&delta(341, 17), &BoardChange::default());

        let save = session.to_save("0.1.0", 4_212);
        let resumed = GameSession::from_save(&save);

        assert_eq!(resumed.last_tick, 17);
        // Neither the log nor the board is persisted — the log is a display
        // buffer and the board is a function of the world, and the next read
        // repopulates both.
        assert!(resumed.settlements().is_empty());
        assert!(resumed.board().is_empty());
    }

    #[test]
    fn the_board_hangs_from_the_world_and_the_digest_names_the_tickers() {
        use crate::data::GameData;
        use mytherra_protocol::{project_world, spectator_standing};

        let rules = GameData::load().unwrap().instruments;
        let content = mytherra_core::data::GameData::load().unwrap();
        let view = project_world(
            &mytherra_core::world::WorldState::new(&content),
            &spectator_standing(),
        );

        let mut session = GameSession::new();
        let opening = session.list(&Snapshot::read(&view), &rules);

        assert!(!session.board().is_empty(), "the world hangs the board");
        assert_eq!(opening.listed.len(), session.board().len());
        assert!(opening.delisted.is_empty());

        // A settlement's digest reports the board's own tickers, so a razed
        // town reads as paper struck off rather than as an entity count.
        let mut razed = view.clone();
        let lost = razed.settlements.remove(0);
        let change = session.list(&Snapshot::read(&razed), &rules);
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
        let config = GameConfig {
            game_name: "tb_realms".to_owned(),
            display_name: "Tradeborn Realms".to_owned(),
            save_slot: "autosave".to_owned(),
            version: "0.2.0".to_owned(),
            server_url: "http://127.0.0.1:8791".to_owned(),
            gateway_url: String::new(),
            spectator_token: String::new(),
            view_poll_seconds: 5.0,
            request_timeout_seconds: 15.0,
        };
        let value = serde_json::json!({ "nonsense": true });

        let migrated = migrate_save_value(Some("0.0.1".to_owned()), value, &config).unwrap();

        assert_eq!(migrated.version, "0.2.0");
        assert_eq!(migrated.cursor, 0);
        assert_eq!(migrated.last_tick, 0);
    }

    #[test]
    fn a_current_save_survives_migration_with_its_cursor_intact() {
        let config = GameConfig {
            game_name: "tb_realms".to_owned(),
            display_name: "Tradeborn Realms".to_owned(),
            save_slot: "autosave".to_owned(),
            version: "0.2.0".to_owned(),
            server_url: "http://127.0.0.1:8791".to_owned(),
            gateway_url: String::new(),
            spectator_token: String::new(),
            view_poll_seconds: 5.0,
            request_timeout_seconds: 15.0,
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
