//! The desk: the session's own state, and the order ticket.
//!
//! The ticket quotes an order using the same function that would execute it, so
//! the number shown before trading is the number the ledger gets. Two code paths
//! there would be a way to lie to the player about their own costs.
//!
//! Slippage and fees are broken out rather than folded into one total, because
//! "your size was the problem" is what a broker needs to be told *before* they
//! place the order, not after.

use super::widgets::{draw_button, panel, signed_percent, truncate};
use super::{UiAction, UiContext, DESK};
use crate::market::orders::Side;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = DESK;
    panel(rect, "Desk");

    draw_ui_text_ex(
        &format!(
            "{} settlements observed · last tick {}",
            ctx.session.observed, ctx.session.last_tick
        ),
        rect.x + 16.0,
        rect.y + 54.0,
        TextStyle::new(14.0, dark::TEXT).params(),
    );
    draw_ui_text_ex(
        "The exchange reads this world and cannot touch it.",
        rect.x + 16.0,
        rect.y + 76.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    // The market's noise stream, seed and all. The same stream plus the same
    // orders reproduces the same century exactly (GDD 5.8), so it is shown
    // rather than hidden — a player can check that the floor is not cheating.
    draw_ui_text_ex(
        &format!(
            "v{} · stream {:016x} from seed {}",
            ctx.data.config.version,
            ctx.session.market_state(),
            ctx.data.config.market.seed
        ),
        rect.x + 16.0,
        rect.y + 97.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    // The tape: the last thing actually traded, at what it actually filled.
    let tape = match ctx.session.portfolio().ledger().first() {
        Some(fill) => format!(
            "last: {} {} {} at {:.2} · {:.0} in friction",
            fill.side.label(),
            fill.units,
            fill.symbol,
            fill.price,
            fill.friction()
        ),
        None => "R read · S save · L load".to_owned(),
    };
    draw_ui_text_ex(
        &tape,
        rect.x + 16.0,
        rect.y + 117.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );

    draw_ticket(ctx, rect, mouse, actions);

    let mut x = rect.right() - 396.0;
    for (label, action, enabled) in [
        ("Read now", UiAction::ReadNow, true),
        ("Save", UiAction::Save, true),
        ("Load", UiAction::Load, ctx.save_exists),
    ] {
        if draw_button(
            Rect::new(x, rect.y + 88.0, 120.0, 34.0),
            label,
            mouse,
            enabled,
        ) {
            actions.push(action);
        }
        x += 132.0;
    }
}

fn draw_ticket(ctx: &UiContext<'_>, desk: Rect, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let x = desk.x + 340.0;
    let Some(listing) = ctx.session.selected() else {
        draw_ui_text_ex(
            "no listing selected — click a row on the board",
            x,
            desk.y + 54.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        return;
    };

    // What is held and what it cost — a broker deciding whether to add should
    // not have to look away from the ticket to see their own basis.
    let holding = match ctx.session.portfolio().position(&listing.entity) {
        Some(position) => format!(
            "holding {} at {:.2}",
            position.units,
            position.average_cost()
        ),
        None => "no position".to_owned(),
    };
    draw_ui_text_ex(
        &format!(
            "{}  {}  ·  {:.2}  ·  {holding}",
            listing.symbol,
            truncate(&listing.name, 18),
            listing.price
        ),
        x,
        desk.y + 52.0,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );

    let units = ctx.session.ticket_units;
    draw_ui_text_ex(
        &format!("{units} units"),
        x + 64.0,
        desk.y + 84.0,
        TextStyle::new(15.0, dark::TEXT).params(),
    );
    for (label, step, at) in [("-", -10, x), ("+", 10, x + 150.0)] {
        if draw_button(Rect::new(at, desk.y + 68.0, 50.0, 24.0), label, mouse, true) {
            actions.push(UiAction::ResizeTicket(step));
        }
    }

    // What it would cost, and why it costs more than the screen price.
    let buy = ctx.session.preview(Side::Buy, &ctx.data.config.orders);
    if let Some((fill, allowed)) = &buy {
        draw_ui_text_ex(
            &format!(
                "buy at {:.2} ({} slip) · fee {:.0} · {:.0} crowns",
                fill.price,
                signed_percent(fill.slippage),
                fill.fee,
                -fill.cash
            ),
            x,
            desk.y + 106.0,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        if let Err(err) = allowed {
            draw_ui_text_ex(
                &truncate(&err.label(), 48),
                x,
                desk.y + 124.0,
                TextStyle::new(12.0, Color::new(0.93, 0.65, 0.30, 1.0)).params(),
            );
        }
    }

    let can_buy = buy.is_some_and(|(_, allowed)| allowed.is_ok());
    let can_sell = ctx
        .session
        .preview(Side::Sell, &ctx.data.config.orders)
        .is_some_and(|(_, allowed)| allowed.is_ok());
    for (side, at, enabled) in [
        (Side::Buy, x + 216.0, can_buy),
        (Side::Sell, x + 314.0, can_sell),
    ] {
        if draw_button(
            Rect::new(at, desk.y + 64.0, 88.0, 34.0),
            side.label(),
            mouse,
            enabled,
        ) {
            actions.push(UiAction::Order(side));
        }
    }
}
