//! The game loop: owns the world source, the session, and the dispatcher that
//! turns UI intents into state changes.
//!
//! The `Game` holds its world behind `Box<dyn WorldSource>` rather than a
//! concrete type, so nothing here knows or cares that the world arrives over a
//! wire (GDD 11). Swapping in an embedded, locally ticked world later is a
//! change to `Game::new` and nothing else in this file.

use crate::data::GameData;
use crate::market::orders::Side;
use crate::market::BoardChange;
use crate::state::{migrate_save_value, GameSession, SaveData, Screen};
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
    /// A scene the capture harness asked for, applied once the world has
    /// answered. Held rather than applied immediately because every scene worth
    /// photographing needs a board, and the board arrives over a wire.
    pending_scene: Option<String>,
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

        let mut session = GameSession::new(&data.config.market, &data.config.orders);
        session.adopt_honours(data.achievements.clone());
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
            pending_scene: None,
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

        self.read_keys();

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }

        self.apply_capture_scene();
    }

    /// Seed a named scene for the screenshot harness (GDD 14).
    ///
    /// Called once before the capture loop; the scene is applied on the first
    /// frame that actually has a board, because everything worth photographing
    /// here is downstream of the world answering.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        self.pending_scene = Some(scene.to_owned());
    }

    /// Put the exchange into the state a capture scene names.
    ///
    /// `trading` is the one that earns its keep: it selects a listing and buys
    /// it, so a screenshot covers the ticket, the fill, and the ledger with a
    /// real position in it — the whole order path, which no static scene shows.
    fn apply_capture_scene(&mut self) {
        let Some(scene) = self.pending_scene.clone() else {
            return;
        };
        if self.session.board().is_empty() {
            return;
        }
        // Both trading scenes start from the deepest listing, so a screenshot is
        // not fighting slippage while it is trying to show something else.
        let deepest = self
            .session
            .board()
            .listings()
            .max_by(|a, b| a.liquidity.total_cmp(&b.liquidity))
            .map(|listing| listing.entity.clone());
        match (scene.as_str(), deepest) {
            ("trading", Some(entity)) => {
                self.session.select(entity);
                self.place_order(Side::Buy);
            }
            ("listing", Some(entity)) => {
                self.session.select(entity.clone());
                self.place_order(Side::Buy);
                self.session.open_listing(entity);
            }
            ("chronicle", _) => self.session.show(Screen::Chronicle),
            ("research", _) => self.session.show(Screen::Research),
            ("records", _) => self.session.show(Screen::Records),
            _ => {}
        }
        self.pending_scene = None;
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

    /// Read the world when its clock has moved past the one the board was hung
    /// from. Returns what changed, which is empty on the overwhelming majority
    /// of frames — the world turns every few seconds and the exchange draws
    /// sixty times a second.
    fn rehang_board(&mut self, turning: Option<&crate::world::WorldDelta>) -> BoardChange {
        let Some(view) = self.world.view() else {
            return BoardChange::default();
        };
        if self.board_tick == Some(view.tick_count) {
            return BoardChange::default();
        }
        self.board_tick = Some(view.tick_count);

        // The world's own back-issues, if this is the first view to arrive.
        let archive = view.chronicle.clone();
        let era = view.era.number;
        let era_name = view.era.name.clone();
        let snapshot = Snapshot::read(view);
        self.session.seed_chronicle(&archive);
        let observed = self.session.observe(snapshot, turning, era, &self.data);

        // An age ending is the largest event in the game and the one nothing
        // could have priced. It gets said in the loudest voice the desk has.
        if observed.cataclysm {
            self.notifications
                .danger(format!("The age turns. {era_name} begins."));
        }

        if observed.books.income > 0.0 {
            self.notifications
                .success(format!("Coupons paid: {:.0} crowns", observed.books.income));
        }
        // A position in something the world stopped carrying is a total loss,
        // not a markdown. It deserves to be said out loud.
        for symbol in &observed.books.written_off {
            self.notifications
                .danger(format!("{symbol} delisted — the position is a total loss"));
        }
        self.announce(&observed.won);
        observed.board
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            // The world turns on the server's clock, so "read now" can only
            // hurry the *reading*, never the world. There is deliberately no
            // way from here to make the world advance.
            UiAction::ReadNow => self.notifications.info("Reading on the next poll"),
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::Select(entity) => self.session.select(entity),
            UiAction::OpenListing(entity) => self.session.open_listing(entity),
            UiAction::CloseListing => self.session.close_listing(),
            UiAction::Show(screen) => self.session.show(screen),
            UiAction::FilterChronicle(kind) => self.session.filter_chronicle(kind),
            UiAction::Research(class) => self.session.research(class),
            UiAction::BuyLicence => self.buy_licence(),
            UiAction::StepSelection(rows) => self.session.step_selection(rows, &self.data.licences),
            UiAction::ResizeTicket(by) => self.session.resize_ticket(by),
            UiAction::Order(side) => self.place_order(side),
        }
    }

    /// Every keyboard route through the exchange (GDD 13, M7 accessibility).
    ///
    /// Everything a mouse can do here, a keyboard can do too: the screens are
    /// numbered, the board is walked with the arrows, and an order is one key.
    /// There is deliberately **no key to advance the world** — the world turns
    /// on its own clock and no input this game accepts can hurry it, which is
    /// Pillar 5 and not an omission.
    fn read_keys(&mut self) {
        for (key, screen) in [
            (KeyCode::Key1, Screen::Floor),
            (KeyCode::Key2, Screen::Chronicle),
            (KeyCode::Key3, Screen::Research),
            (KeyCode::Key4, Screen::Records),
        ] {
            if is_key_pressed(key) {
                self.events.push(UiAction::Show(screen));
            }
        }

        // Walking the board with the arrows, which is the one interaction a
        // mouse-only design would leave a keyboard player unable to perform.
        if is_key_pressed(KeyCode::Down) {
            self.events.push(UiAction::StepSelection(1));
        }
        if is_key_pressed(KeyCode::Up) {
            self.events.push(UiAction::StepSelection(-1));
        }
        if is_key_pressed(KeyCode::Enter) {
            if let Some(listing) = self.session.selected() {
                let entity = listing.entity.clone();
                self.events.push(UiAction::OpenListing(entity));
            }
        }

        if is_key_pressed(KeyCode::B) {
            self.events.push(UiAction::Order(Side::Buy));
        }
        // Sell is X rather than S, which is already the save key and was here
        // first.
        if is_key_pressed(KeyCode::X) {
            self.events.push(UiAction::Order(Side::Sell));
        }
        if is_key_pressed(KeyCode::LeftBracket) {
            self.events.push(UiAction::ResizeTicket(-10));
        }
        if is_key_pressed(KeyCode::RightBracket) {
            self.events.push(UiAction::ResizeTicket(10));
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
        // Escape backs out of a listing, which is the one navigation a player
        // should never have to hunt for a button to perform.
        if is_key_pressed(KeyCode::Escape) {
            self.events.push(UiAction::CloseListing);
        }
    }

    /// Say what the exchange has noticed. The reward is the noticing.
    fn announce(&mut self, won: &[String]) {
        for id in won {
            let name = self
                .session
                .honours()
                .get(id)
                .map(|honour| honour.name.clone())
                .unwrap_or_else(|| id.clone());
            self.notifications.success(format!("Noted: {name}"));
        }
    }

    /// Climb one rung of the information ladder (GDD 5.3).
    fn buy_licence(&mut self) {
        match self.session.buy_licence(&self.data.licences) {
            Ok(name) => self
                .notifications
                .success(format!("Licensed: {name}. The board widens.")),
            Err(err) => self.notifications.warning(err.label()),
        }
    }

    fn place_order(&mut self, side: Side) {
        match self.session.order(side, &self.data.config.orders) {
            Ok(fill) => self.notifications.success(format!(
                "{} {} {} at {:.2} — {:.0} crowns, {:.0} in friction",
                side.label(),
                fill.units,
                fill.symbol,
                fill.price,
                fill.cash.abs(),
                fill.friction()
            )),
            Err(err) => self.notifications.warning(err.label()),
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
                self.session = GameSession::from_save(
                    &save,
                    &self.data.config.market,
                    &self.data.config.orders,
                );
                // A save may predate an achievement, so the shipped definitions
                // are re-adopted rather than trusted from the file.
                self.session.adopt_honours(self.data.achievements.clone());
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
