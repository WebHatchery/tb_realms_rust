//! The settlement log: the world's turnings, as the exchange saw them.
//!
//! The chronicle line beneath each record is the newspaper (GDD 5.5) — grand,
//! mythic, and read by nothing that prices. The numbers above it are the truth.
//! The gap between the two is most of the game's sense of humour.

use super::widgets::{panel, truncate};
use super::{UiContext, SETTLEMENTS};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(ctx: &UiContext<'_>) {
    let rect = SETTLEMENTS;
    panel(rect, "Settlements");

    if ctx.session.settlements().is_empty() {
        draw_ui_text_ex(
            "the world has not turned yet",
            rect.x + 14.0,
            rect.y + 56.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = rect.y + 54.0;
    for record in ctx.session.settlements().iter().take(12) {
        let span = if record.years > 1 {
            format!("Year {} (+{})", record.year, record.years)
        } else {
            format!("Year {}", record.year)
        };
        draw_ui_text_ex(
            &span,
            rect.x + 14.0,
            y,
            TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
        );

        // Listings and delistings are the ones no authored market produces, so
        // they are called out rather than folded into the move count.
        let mut summary = format!("{} moves", record.moves);
        if record.listed > 0 {
            summary.push_str(&format!(" · {} up", record.listed));
        }
        if record.delisted > 0 {
            summary.push_str(&format!(" · {} off", record.delisted));
        }
        draw_ui_text_ex(
            &summary,
            rect.x + 106.0,
            y,
            TextStyle::new(12.0, dark::TEXT).params(),
        );
        y += 17.0;

        for line in &record.notable {
            draw_ui_text_ex(
                &truncate(line, rect.w - 34.0, 12.0),
                rect.x + 20.0,
                y,
                TextStyle::new(12.0, Color::new(0.85, 0.72, 0.45, 1.0)).params(),
            );
            y += 16.0;
        }

        // Why the year moved, in the same vocabulary the balance tables use.
        if !record.top_moves.is_empty() {
            draw_ui_text_ex(
                &truncate(&record.top_moves.join(" · "), rect.w - 34.0, 12.0),
                rect.x + 20.0,
                y,
                TextStyle::new(12.0, dark::TEXT).params(),
            );
            y += 16.0;
        }

        if let Some(headline) = &record.headline {
            draw_ui_text_ex(
                &truncate(headline, rect.w - 34.0, 12.0),
                rect.x + 20.0,
                y,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            y += 16.0;
        }
        y += 5.0;
        if y > rect.bottom() - 20.0 {
            break;
        }
    }
}
