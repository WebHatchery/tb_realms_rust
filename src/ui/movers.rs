//! The price-move decomposition (GDD 5.4) — Pillar 1 made visible.
//!
//! Every term that pushed a price is named in crowns, and the terms sum to the
//! move. A player must always be able to ask why a price moved and get an
//! answer; this panel is where the answer is kept.

use super::widgets::{direction, panel, signed_percent, truncate};
use super::{UiContext, MOVERS};
use crate::market::Instrument;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(ctx: &UiContext<'_>) {
    let rect = MOVERS;
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
            rect.x + 14.0,
            rect.y + 56.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
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
            rect.x + 14.0,
            y,
            TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &truncate(&listing.name, rect.w - 134.0, 14.0),
            rect.x + 66.0,
            y,
            TextStyle::new(14.0, dark::TEXT).params(),
        );
        draw_ui_text_ex(
            &signed_percent(change),
            rect.right() - 58.0,
            y,
            TextStyle::new(14.0, direction(change)).params(),
        );
        y += 17.0;

        // What it is a claim on, named — Pillar 1 again: never a free-floating
        // number. `depth` is the class's liquidity: how much paper the floor
        // will actually take, which is why a relic lot is not a bond.
        draw_ui_text_ex(
            &truncate(
                &format!(
                    "{} · listed {} · depth {:.0}%",
                    listing.class,
                    listing.listed_year,
                    listing.liquidity * 100.0
                ),
                rect.w - 34.0,
                11.0,
            ),
            rect.x + 20.0,
            y,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
        y += 16.0;

        let parts: Vec<String> = listing
            .last_move
            .parts()
            .into_iter()
            .map(|(name, value)| format!("{name} {value:+.2}"))
            .collect();
        draw_ui_text_ex(
            &truncate(&parts.join("  "), rect.w - 34.0, 12.0),
            rect.x + 20.0,
            y,
            TextStyle::new(12.0, dark::TEXT).params(),
        );
        y += 21.0;
        if y > rect.bottom() - 18.0 {
            break;
        }
    }
}
