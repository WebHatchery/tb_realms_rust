//! The exchange's boards.
//!
//! This is a **pure view layer** (GDD 9): it reads state, draws, and returns
//! [`UiAction`] intents. It never mutates the session and never touches the
//! world — a panel that wants something adds a variant here and the dispatcher
//! in `game.rs` decides what it means.
//!
//! What it draws today is the board the world hangs — every listing, its price,
//! what the fundamentals say it is worth, and the gap between the two — beside
//! the movers with their price moves decomposed, and the settlements as they
//! land. Charts, order tickets and the research browser are M3.
//!
//! The decomposition panel is Pillar 1 made visible: ask why a price moved and
//! the answer is drift, shock and noise in crowns, never a shrug.

use crate::data::GameData;
use crate::market::Instrument;
use crate::state::GameSession;
use crate::world::SourceStatus;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use macroquad_toolkit::ui::VirtualUi;
use mytherra_protocol::WorldView;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

/// What the player asked the exchange to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    /// Read the world now rather than waiting out the poll interval.
    ReadNow,
    Save,
    Load,
}

pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub session: &'a GameSession,
    pub status: &'a SourceStatus,
    pub view: Option<&'a WorldView>,
    pub save_exists: bool,
    pub ui: &'a VirtualUi,
}

pub fn draw_game_ui(ctx: UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mouse = ctx.ui.mouse_position();

    draw_header(&ctx);
    draw_listings_panel(&ctx);
    draw_movers_panel(&ctx);
    draw_settlements_panel(&ctx);
    draw_footer(&ctx, mouse, &mut actions);

    actions
}

fn panel(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);
    draw_ui_text_ex(
        title,
        rect.x + 16.0,
        rect.y + 26.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
}

fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 76.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);

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

/// The dearest listings first, which is the order a board is read in.
fn ranked_listings(session: &GameSession) -> Vec<&Instrument> {
    let mut ranked: Vec<&Instrument> = session.board().listings().collect();
    ranked.sort_by(|a, b| b.fair_value.total_cmp(&a.fair_value));
    ranked
}

fn draw_listings_panel(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 104.0, 640.0, 452.0);
    panel(rect, "The board");

    if ctx.session.board().is_empty() {
        draw_ui_text_ex(
            "no world in hand yet — nothing is listed",
            rect.x + 16.0,
            rect.y + 60.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let ranked = ranked_listings(ctx.session);
    let price_x = rect.right() - 268.0;
    let change_x = rect.right() - 206.0;
    let fair_x = rect.right() - 142.0;
    let premium_x = rect.right() - 82.0;
    let yield_x = rect.right() - 30.0;
    let mut y = rect.y + 52.0;
    for (label, x) in [
        ("SYM", rect.x + 16.0),
        ("BACKED BY", rect.x + 72.0),
        ("PRICE", price_x),
        ("CHG", change_x),
        ("FAIR", fair_x),
        ("PREM", premium_x),
        ("YIELD", yield_x),
    ] {
        draw_ui_text_ex(label, x, y, TextStyle::new(12.0, dark::TEXT_DIM).params());
    }
    y += 20.0;

    let rows = ((rect.bottom() - 44.0 - y) / 24.0) as usize;
    for listing in ranked.iter().take(rows) {
        draw_ui_text_ex(
            &listing.symbol,
            rect.x + 16.0,
            y,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &truncate(&listing.name, 22),
            rect.x + 72.0,
            y,
            TextStyle::new(14.0, dark::TEXT).params(),
        );
        draw_ui_text_ex(
            &format!("{:.1}", listing.price),
            price_x,
            y,
            TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
        );

        let change = listing.last_move.change_fraction();
        draw_ui_text_ex(
            &signed_percent(change),
            change_x,
            y,
            TextStyle::new(14.0, direction(change)).params(),
        );
        draw_ui_text_ex(
            &format!("{:.1}", listing.fair_value),
            fair_x,
            y,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        // The premium is the number a broker is actually trading: how far the
        // floor sits from what the world says the claim is worth.
        let premium = listing.premium();
        draw_ui_text_ex(
            &signed_percent(premium),
            premium_x,
            y,
            // Coloured by whether it is *cheap*, not by whether it is up: a
            // listing trading under fair value is the buy, so the sign the
            // broker wants is the opposite of the one the price wants.
            TextStyle::new(13.0, direction(-premium)).params(),
        );

        // A listing that pays nothing says so plainly rather than printing a
        // zero that reads like a collapsed coupon.
        let (paid, tint) = if listing.income > 0.0 {
            (format!("{:.1}%", listing.yield_rate() * 100.0), dark::TEXT)
        } else {
            ("—".to_owned(), dark::TEXT_DIM)
        };
        draw_ui_text_ex(&paid, yield_x, y, TextStyle::new(13.0, tint).params());
        y += 24.0;
    }

    let listed = ctx.session.board().len();
    let hidden = listed.saturating_sub(rows);
    let footer = if hidden > 0 {
        format!("{listed} listings · {hidden} more below the fold")
    } else {
        format!("{listed} listings")
    };
    draw_ui_text_ex(
        &footer,
        rect.x + 16.0,
        rect.bottom() - 20.0,
        TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
    );
}

/// What moved furthest this settlement, and why (GDD 5.4).
///
/// This is the price-move decomposition: every term that pushed the price is
/// named in crowns, and the terms sum to the move. A player must always be able
/// to ask why and get an answer — Pillar 1 does not permit a shrug.
fn draw_movers_panel(ctx: &UiContext<'_>) {
    let rect = Rect::new(674.0, 104.0, LOGICAL_WIDTH - 692.0, 236.0);
    panel(rect, "Why it moved");

    let mut movers: Vec<&Instrument> = ctx
        .session
        .board()
        .listings()
        .filter(|listing| listing.last_move.change() != 0.0)
        .collect();
    if movers.is_empty() {
        draw_ui_text_ex(
            "nothing has moved — the world has not turned yet",
            rect.x + 16.0,
            rect.y + 56.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        return;
    }
    movers.sort_by(|a, b| {
        b.last_move
            .change_fraction()
            .abs()
            .total_cmp(&a.last_move.change_fraction().abs())
    });

    let mut y = rect.y + 54.0;
    for listing in movers.iter().take(4) {
        let change = listing.last_move.change_fraction();
        draw_ui_text_ex(
            &listing.symbol,
            rect.x + 16.0,
            y,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &truncate(&listing.name, 24),
            rect.x + 72.0,
            y,
            TextStyle::new(14.0, dark::TEXT).params(),
        );
        draw_ui_text_ex(
            &signed_percent(change),
            rect.right() - 62.0,
            y,
            TextStyle::new(14.0, direction(change)).params(),
        );
        y += 18.0;

        // What it is a claim on, named — Pillar 1 again: never a free-floating
        // number. `depth` is the class's liquidity: how much paper the floor
        // will actually take, which is why a relic lot is not a bond.
        draw_ui_text_ex(
            &truncate(
                &format!(
                    "{} · {} · listed {} · depth {:.0}%",
                    listing.class,
                    listing.entity.id(),
                    listing.listed_year,
                    listing.liquidity * 100.0
                ),
                62,
            ),
            rect.x + 24.0,
            y,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        y += 17.0;

        let parts: Vec<String> = listing
            .last_move
            .parts()
            .into_iter()
            .map(|(name, value)| format!("{name} {value:+.2}"))
            .collect();
        draw_ui_text_ex(
            &truncate(&parts.join("   "), 62),
            rect.x + 24.0,
            y,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        y += 22.0;
        if y > rect.bottom() - 20.0 {
            break;
        }
    }
}

fn draw_settlements_panel(ctx: &UiContext<'_>) {
    let rect = Rect::new(674.0, 348.0, LOGICAL_WIDTH - 692.0, 208.0);
    panel(rect, "Settlements");

    if ctx.session.settlements().is_empty() {
        draw_ui_text_ex(
            "the world has not turned since the exchange opened",
            rect.x + 16.0,
            rect.y + 60.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = rect.y + 58.0;
    for record in ctx.session.settlements().iter().take(12) {
        let span = if record.years > 1 {
            format!("Year {} (+{})", record.year, record.years)
        } else {
            format!("Year {}", record.year)
        };
        draw_ui_text_ex(
            &span,
            rect.x + 16.0,
            y,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );

        // Listings and delistings are the ones no authored market produces, so
        // they are called out rather than folded into the move count.
        let mut summary = format!("{} moves", record.moves);
        if record.listed > 0 {
            summary.push_str(&format!(" · {} listed", record.listed));
        }
        if record.delisted > 0 {
            summary.push_str(&format!(" · {} delisted", record.delisted));
        }
        draw_ui_text_ex(
            &summary,
            rect.x + 140.0,
            y,
            TextStyle::new(14.0, dark::TEXT).params(),
        );
        y += 20.0;

        for line in &record.notable {
            draw_ui_text_ex(
                &truncate(line, 74),
                rect.x + 24.0,
                y,
                TextStyle::new(13.0, Color::new(0.85, 0.72, 0.45, 1.0)).params(),
            );
            y += 18.0;
        }

        // Why the year moved, in the same vocabulary the balance tables use.
        if !record.top_moves.is_empty() {
            draw_ui_text_ex(
                &truncate(&record.top_moves.join("  ·  "), 76),
                rect.x + 24.0,
                y,
                TextStyle::new(13.0, dark::TEXT).params(),
            );
            y += 18.0;
        }

        if let Some(headline) = &record.headline {
            draw_ui_text_ex(
                &truncate(headline, 74),
                rect.x + 24.0,
                y,
                TextStyle::new(13.0, dark::TEXT_DIM).params(),
            );
            y += 18.0;
        }
        y += 6.0;
        if y > rect.bottom() - 24.0 {
            break;
        }
    }
}

fn draw_footer(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(18.0, 568.0, LOGICAL_WIDTH - 36.0, 134.0);
    panel(rect, "Desk");

    draw_ui_text_ex(
        &format!(
            "{} settlements observed · last tick {}",
            ctx.session.observed, ctx.session.last_tick
        ),
        rect.x + 16.0,
        rect.y + 56.0,
        TextStyle::new(15.0, dark::TEXT).params(),
    );
    // The market's noise stream, seed and all. Same stream plus the same orders
    // reproduces the same century exactly (GDD 5.8), so it is shown rather than
    // hidden — a player can check that the floor is not cheating.
    draw_ui_text_ex(
        &format!(
            "market stream {:016x} from seed {}",
            ctx.session.market_state(),
            ctx.data.config.market.seed
        ),
        rect.x + 340.0,
        rect.y + 56.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        "The exchange reads this world and cannot touch it.",
        rect.x + 16.0,
        rect.y + 80.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        &format!(
            "v{}  ·  R read now  ·  S save  ·  L load",
            ctx.data.config.version
        ),
        rect.x + 16.0,
        rect.y + 104.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let mut x = rect.right() - 396.0;
    for (label, action, enabled) in [
        ("Read now", UiAction::ReadNow, true),
        ("Save", UiAction::Save, true),
        ("Load", UiAction::Load, ctx.save_exists),
    ] {
        let button_rect = Rect::new(x, rect.y + 48.0, 120.0, 40.0);
        if draw_button(button_rect, label, mouse, enabled) {
            actions.push(action);
        }
        x += 132.0;
    }
}

/// A flat button. Returns true on release inside it, so a press that drags away
/// is not a click.
fn draw_button(rect: Rect, label: &str, mouse: Vec2, enabled: bool) -> bool {
    let hovered = enabled && rect.contains(mouse);
    let fill = if !enabled {
        Color::new(0.12, 0.13, 0.16, 1.0)
    } else if hovered {
        Color::new(0.22, 0.30, 0.40, 1.0)
    } else {
        Color::new(0.16, 0.19, 0.24, 1.0)
    };
    let text = if enabled {
        dark::TEXT_BRIGHT
    } else {
        dark::TEXT_DIM
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_border(1.0, dark::ACCENT),
    );
    let width = measure_text(label, None, 16, 1.0).width;
    draw_ui_text_ex(
        label,
        rect.x + (rect.w - width) / 2.0,
        rect.y + rect.h / 2.0 + 6.0,
        TextStyle::new(16.0, text).params(),
    );
    hovered && is_mouse_button_released(MouseButton::Left)
}

/// A move as a signed percentage, or a dash when it did not really move.
///
/// The sign is always printed. That is deliberate: it makes direction legible
/// without colour, which is the accessibility half of the pair below.
fn signed_percent(fraction: f32) -> String {
    if fraction.abs() < 0.0005 {
        return "—".to_owned();
    }
    format!("{:+.1}%", fraction * 100.0)
}

/// Up and down, told apart by more than colour (GDD 12, open question 5).
///
/// Red/green is the market convention and the worst possible choice for
/// colour-blind players — those are precisely the two hues a red-green
/// deficiency cannot separate. So this is teal against amber, which stays
/// distinguishable under every common deficiency, and the sign printed beside it
/// carries the meaning on its own regardless.
fn direction(change: f32) -> Color {
    if change > 0.0005 {
        Color::new(0.36, 0.78, 0.72, 1.0)
    } else if change < -0.0005 {
        Color::new(0.93, 0.65, 0.30, 1.0)
    } else {
        dark::TEXT_DIM
    }
}

/// Clip a line to `max` characters, on a character boundary so multi-byte
/// glyphs in the chronicle's prose can't be split mid-encoding.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_lines_and_clips_long_ones() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 10), "abcdefghij");
        assert_eq!(truncate("abcdefghijk", 10), "abcdefghi…");
    }

    #[test]
    fn truncate_splits_on_character_boundaries() {
        // The chronicle is authored prose and may carry non-ASCII; slicing by
        // byte would panic here.
        let line = "æther — the Arcane Circle rises in Duskvale";
        let clipped = truncate(line, 12);
        assert_eq!(clipped.chars().count(), 12);
        assert!(clipped.ends_with('…'));
    }
}
