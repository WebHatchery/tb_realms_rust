//! The board: every listing the world backs, and the row you click to trade it.
//!
//! Sorted by fair value, which is how a board is read — the substantial paper
//! at the top. The column a broker actually works from is `PREM`: how far the
//! floor's price sits from what the world says the claim is worth.

use super::widgets::{direction, panel, signed_percent, truncate};
use super::{UiAction, UiContext, BOARD};
use crate::data::GameData;
use crate::market::Instrument;
use crate::state::GameSession;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

/// The dearest listings a broker of this rank may see, dearest first.
///
/// Paper outside the licence is not greyed out but *absent*: a broker who
/// cannot trade a class should not be teased with its prices, because the whole
/// point of the ladder is that information is the thing being bought (Pillar 3).
fn ranked<'a>(session: &'a GameSession, data: &GameData) -> Vec<&'a Instrument> {
    let held = session.licences().held;
    let mut listings: Vec<&Instrument> = session
        .board()
        .listings()
        .filter(|listing| {
            listing
                .entity
                .listing_class()
                .is_some_and(|class| data.licences.permits(held, class))
        })
        .collect();
    listings.sort_by(|a, b| b.fair_value.total_cmp(&a.fair_value));
    listings
}

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = BOARD;
    panel(rect, "The board");

    let listings = ranked(ctx.session, ctx.data);
    if listings.is_empty() {
        draw_ui_text_ex(
            if ctx.session.board().is_empty() {
                "no world in hand yet — nothing is listed"
            } else {
                "your licence shows you nothing the world is currently offering"
            },
            rect.x + 16.0,
            rect.y + 60.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let selected = ctx.session.selected().map(|listing| &listing.entity);
    let price_x = rect.right() - 214.0;
    let change_x = rect.right() - 156.0;
    let premium_x = rect.right() - 96.0;
    let yield_x = rect.right() - 34.0;

    let mut y = rect.y + 52.0;
    for (label, x) in [
        ("SYM", rect.x + 14.0),
        ("BACKED BY", rect.x + 70.0),
        ("PRICE", price_x),
        ("CHG", change_x),
        ("PREM", premium_x),
        ("YIELD", yield_x),
    ] {
        draw_ui_text_ex(label, x, y, TextStyle::new(12.0, dark::TEXT_DIM).params());
    }
    y += 20.0;

    let rows = ((rect.bottom() - 44.0 - y) / 24.0) as usize;
    for listing in listings.iter().take(rows) {
        // The whole row is the target — a ticker four characters wide is not a
        // click target anyone should have to hit.
        let row = Rect::new(rect.x + 8.0, y - 15.0, rect.w - 16.0, 22.0);
        let hovered = row.contains(mouse);
        if selected == Some(&listing.entity) {
            draw_surface(row, &SurfaceStyle::new(Color::new(0.16, 0.22, 0.30, 1.0)));
        } else if hovered {
            draw_surface(row, &SurfaceStyle::new(Color::new(0.12, 0.14, 0.18, 1.0)));
        }
        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::Select(listing.entity.clone()));
        }

        draw_ui_text_ex(
            &listing.symbol,
            rect.x + 14.0,
            y,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );

        // A holding is marked on the board itself, so a broker can see their own
        // book without leaving the page they trade from.
        let held = ctx.session.portfolio().units_of(&listing.entity);
        let name = if held > 0 {
            format!("{} ({held})", truncate(&listing.name, 13))
        } else {
            truncate(&listing.name, 19)
        };
        draw_ui_text_ex(
            &name,
            rect.x + 70.0,
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

    let listed = listings.len();
    let hidden = listed.saturating_sub(rows);
    // What the licence is keeping from you, counted. Knowing how much you
    // cannot see is itself information, and it is the sales pitch for the next
    // rung of the ladder.
    let unlicensed = ctx.session.board().len() - listed;
    let mut footer = format!("{listed} listings");
    if hidden > 0 {
        footer.push_str(&format!(" · {hidden} below the fold"));
    }
    if unlicensed > 0 {
        footer.push_str(&format!(" · {unlicensed} beyond your licence"));
    }
    draw_ui_text_ex(
        &footer,
        rect.x + 16.0,
        rect.bottom() - 20.0,
        TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
    );
}
