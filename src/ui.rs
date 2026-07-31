//! The exchange's screens.
//!
//! This is a **pure view layer** (GDD 9): it reads state, draws, and returns
//! [`UiAction`] intents. It never mutates the session and never touches the
//! world — a panel that wants something adds a variant here and the dispatcher
//! in `game.rs` decides what it means.
//!
//! This file owns only the vocabulary and the layout: what a panel may ask for,
//! what it may read, and where each one sits. The panels themselves live in
//! `ui/`, one file each, so a screen can grow without the others noticing.
//!
//! Two of them carry a design promise rather than just pixels. The
//! decomposition panel is Pillar 1 made visible — ask why a price moved and the
//! answer is drift, shock and noise in crowns, never a shrug. The ticket is the
//! same promise about your own trading: what it quotes is what the order will
//! cost, because the quote and the execution are one function.

mod board;
mod chart;
mod desk;
mod ledger;
mod listing;
mod movers;
mod settlements;
mod widgets;

use crate::data::GameData;
use crate::market::orders::Side;
use crate::state::{GameSession, Screen};
use crate::world::fundamentals::EntityRef;
use crate::world::SourceStatus;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::VirtualUi;
use mytherra_protocol::WorldView;
use widgets::{panel, truncate};

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

/// What the player asked the exchange to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    /// Read the world now rather than waiting out the poll interval.
    ReadNow,
    Save,
    Load,
    /// Look at a listing. The desk's ticket follows the selection.
    Select(EntityRef),
    /// Open a listing's own page: its chart, its fundamentals, its last move.
    OpenListing(EntityRef),
    /// Return to the floor.
    CloseListing,
    /// Change the ticket's size by this many units.
    ResizeTicket(i64),
    /// Place the ticket as it stands.
    Order(Side),
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub status: &'a SourceStatus,
    pub view: Option<&'a WorldView>,
    pub save_exists: bool,
    pub ui: &'a VirtualUi,
}

/// Where each panel sits. Kept together so the layout can be read in one place
/// rather than reconstructed from six files.
pub(crate) const BOARD: Rect = Rect {
    x: 18.0,
    y: 104.0,
    w: 582.0,
    h: 452.0,
};
pub(crate) const LEDGER: Rect = Rect {
    x: 610.0,
    y: 104.0,
    w: 330.0,
    h: 452.0,
};
pub(crate) const MOVERS: Rect = Rect {
    x: 950.0,
    y: 104.0,
    w: 312.0,
    h: 224.0,
};
pub(crate) const SETTLEMENTS: Rect = Rect {
    x: 950.0,
    y: 336.0,
    w: 312.0,
    h: 220.0,
};
pub(crate) const DESK: Rect = Rect {
    x: 18.0,
    y: 568.0,
    w: LOGICAL_WIDTH - 36.0,
    h: 134.0,
};

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mouse = ctx.ui.mouse_position();

    // The header is common to both screens: the world's clock and the state of
    // the wire are true wherever the player is standing.
    draw_header(&ctx);
    match ctx.session.screen() {
        Screen::Floor => {
            board::draw(&ctx, mouse, &mut actions);
            ledger::draw(&ctx);
            movers::draw(&ctx);
            settlements::draw(&ctx);
            desk::draw(&ctx, mouse, &mut actions);
        }
        Screen::Listing => listing::draw(&ctx, mouse, &mut actions),
    }

    actions
}

fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 76.0);
    panel(rect, "");

    draw_ui_text_ex(
        &ctx.data.config.display_name,
        rect.x + 18.0,
        rect.y + 34.0,
        TextStyle::new(28.0, dark::TEXT_BRIGHT).params(),
    );

    // The world's own clock, which the exchange does not control.
    let clock = match ctx.view {
        Some(view) => format!(
            "Year {} · {} · tick {}",
            view.year, view.era.name, view.tick_count
        ),
        None => "the world has not yet answered".to_owned(),
    };
    draw_ui_text_ex(
        &clock,
        rect.x + 18.0,
        rect.y + 60.0,
        TextStyle::new(16.0, dark::TEXT).params(),
    );

    // The state of the wire, coloured by whether the exchange can see anything.
    let (tint, text) = match ctx.status {
        SourceStatus::Connecting => (Color::new(0.75, 0.68, 0.35, 1.0), ctx.status.label()),
        SourceStatus::Live => (Color::new(0.40, 0.78, 0.52, 1.0), ctx.status.label()),
        SourceStatus::Failed(_) => (Color::new(0.85, 0.42, 0.42, 1.0), ctx.status.label()),
    };
    let width = 380.0;
    draw_badge(
        Rect::new(rect.right() - width - 18.0, rect.y + 20.0, width, 26.0),
        &truncate(&text, 52),
        Color::new(0.14, 0.16, 0.20, 1.0),
        tint,
    );
    draw_ui_text_ex(
        &format!("reading {}", truncate(&ctx.data.config.world_url(), 58)),
        rect.right() - width - 18.0,
        rect.y + 64.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
}
