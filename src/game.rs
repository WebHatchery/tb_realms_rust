//! The game loop: owns the world source, the session, and the dispatcher that
//! turns UI intents into state changes.
//!
//! The `Game` holds its world behind `Box<dyn WorldSource>` rather than a
//! concrete type, so nothing here knows or cares that the world arrives over a
//! wire (GDD 11). Swapping in an embedded, locally ticked world later is a
//! change to `Game::new` and nothing else in this file.

use crate::data::GameData;
use crate::market::BoardChange;
use crate::state::{migrate_save_value, GameSession, SaveData};
use crate::ui::{self, UiAction, UiContext};
use crate::world::live::{LiveWorld, SpectatorClient};
use crate::world::{Snapshot, SourceStatus, WorldSource};
use macroquad::prelude::*;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    load_from_slot_with_migration, save_to_slot_with_version, slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};

pub struct Game {
    data: GameData,
    session: GameSession,
    world: Box<dyn WorldSource>,
    /// The chronicle cursor, mirrored here so it can be saved. The source owns
    /// the authoritative one; this follows it.
    cursor: u64,
    /// The world tick the board was last hung from. The board is rebuilt when
    /// this falls behind the world's own clock — which covers both the first
    /// view to arrive and every turning after it — rather than every frame.
    board_tick: Option<u64>,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    save_exists: bool,
    /// Set when the exchange could not be opened at all — a missing spectator
    /// token, say. Rendered as a dead wire rather than silently polling nothing.
    fatal: Option<SourceStatus>,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("Tradeborn Realms embedded data failed to load: {err}");
        });

        let mut notifications = NotificationManager::new();

        // A missing token is a configuration failure, not a network one, and
        // saying so plainly here saves a long hunt through 401s later.
        let (world, fatal): (Box<dyn WorldSource>, Option<SourceStatus>) =
            match data.config.spectator_token() {
                Ok(token) => {
                    let client = SpectatorClient::new(data.config.world_url(), token);
                    notifications.info(format!("Reading the world at {}", data.config.world_url()));
                    (
                        Box::new(LiveWorld::connect(
                            client,
                            data.config.view_poll_seconds,
                            data.config.request_timeout_seconds,
                        )),
                        None,
                    )
                }
                Err(err) => {
                    notifications.danger(err.clone());
                    (Box::new(NoWorld), Some(SourceStatus::Failed(err)))
                }
            };

        let session = GameSession::new(&data.config.market);
        let mut game = Self {
            data,
            session,
            world,
            cursor: 0,
            board_tick: None,
            notifications,
            events: EventBus::new(),
            save_exists: false,
            fatal,
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);

        // The world turns on its own schedule; the exchange only reads it.
        let turning = self.world.poll(dt);
        let board = self.rehang_board(turning.as_ref());

        if let Some(delta) = turning {
            self.session.record(&delta, &board);
            // Follow the source's cursor so a save records how far the exchange
            // has actually read.
            self.cursor = self.world.cursor();

            if !board.listed.is_empty() || !board.delisted.is_empty() {
                self.notifications.info(format!(
                    "Year {}: {} listed, {} delisted",
                    delta.to_year,
                    board.listed.len(),
                    board.delisted.len()
                ));
            } else if delta.is_empty() {
                self.notifications
                    .info(format!("Year {}: a quiet year", delta.to_year));
            } else {
                self.notifications.info(format!(
                    "Year {}: {} moves across {}y",
                    delta.to_year,
                    delta.moves.len(),
                    delta.years()
                ));
            }
        }

        if is_key_pressed(KeyCode::R) {
            self.events.push(UiAction::ReadNow);
        }
        if is_key_pressed(KeyCode::S) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::L) {
            self.events.push(UiAction::Load);
        }

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let ctx = UiContext {
            data: &self.data,
            session: &self.session,
            status: self.fatal.as_ref().unwrap_or_else(|| self.world.status()),
            view: self.world.view(),
            save_exists: self.save_exists,
            ui: &virtual_ui,
        };

        let actions = ui::draw_game_ui(ctx);
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
    }

    /// Rebuild the board when the world's clock has moved past the one it was
    /// hung from, and settle every price if the world actually turned. Returns
    /// what changed, which is empty on the overwhelming majority of frames — the
    /// world turns every few seconds and the exchange draws sixty times a
    /// second.
    ///
    /// The order matters. Listings are refreshed first so a settlement prices
    /// against the fair values the *new* world implies, and only then do prices
    /// move toward them. Settling first would drift every price toward a fair
    /// value one year stale.
    ///
    /// `turning` is `None` on the first view to arrive: the board hangs, but
    /// nothing settles, because no year has passed for the exchange to price.
    fn rehang_board(&mut self, turning: Option<&crate::world::WorldDelta>) -> BoardChange {
        let Some(view) = self.world.view() else {
            return BoardChange::default();
        };
        if self.board_tick == Some(view.tick_count) {
            return BoardChange::default();
        }
        self.board_tick = Some(view.tick_count);

        let snapshot = Snapshot::read(view);
        let change = self
            .session
            .list(&snapshot, &self.data.instruments, &self.data.config.market);
        if let Some(delta) = turning {
            self.session.settle_prices(
                delta,
                &snapshot,
                &self.data.impacts,
                &self.data.config.market,
            );
        }
        change
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            // The world turns on the server's clock, so "read now" can only
            // hurry the *reading*, never the world. There is deliberately no
            // way from here to make the world advance.
            UiAction::ReadNow => self.notifications.info("Reading on the next poll"),
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
        }
    }

    fn save_game(&mut self) {
        let save = self.session.to_save(&self.data.config.version, self.cursor);
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications.success("Ledger saved");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.danger(format!("Save failed: {err}")),
        }
    }

    fn load_game(&mut self) {
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &self.data.config.version,
            |version, value| migrate_save_value(version, value, &self.data.config),
        );

        match loaded {
            Ok(save) => {
                self.cursor = save.cursor;
                // Tell the world source too, or the next read would re-serve
                // every chronicle line the exchange had already read.
                self.world.resume_from(save.cursor);
                self.session = GameSession::from_save(&save, &self.data.config.market);
                self.notifications.success("Ledger loaded");
                self.refresh_save_state();
            }
            Err(err) => self.notifications.warning(format!("Load failed: {err}")),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
    }
}

/// The world source used when the exchange could not be opened at all. It
/// exists so the rest of the game has no "maybe there is no world" branch to
/// carry — the boards render, the banner explains, and nothing else changes.
struct NoWorld;

impl WorldSource for NoWorld {
    fn poll(&mut self, _dt: f32) -> Option<crate::world::WorldDelta> {
        None
    }

    fn view(&self) -> Option<&mytherra_protocol::WorldView> {
        None
    }

    fn cursor(&self) -> u64 {
        0
    }

    fn resume_from(&mut self, _cursor: u64) {}

    fn status(&self) -> &SourceStatus {
        // A source that was never opened is not "connecting" — it is down, and
        // the reason is on the banner the caller supplies.
        &SourceStatus::Connecting
    }
}
