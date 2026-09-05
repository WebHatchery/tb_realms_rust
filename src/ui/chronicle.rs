//! The chronicle: the world in its own words, beside what actually moved.
//!
//! **Pillar 4 is the whole layout of this screen.** The prose is the newspaper;
//! it tells you what happened in the world's voice, grand and mythic. The strip
//! of tickers beside each year is what the exchange actually did about it. The
//! two are printed together and *nothing joins them but the year* — the price
//! oracle never reads a word of this (GDD 5.5), so any correspondence a player
//! sees is one they inferred themselves.
//!
//! That is deliberate, and it is where the game keeps its sense of humour: a
//! year whose chronicle records the fall of a great house, beside a ticker strip
//! that shrugged and moved grain by four percent.

use super::widgets::{direction, panel, signed_percent, truncate};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::state::SettlementRecord;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;
use mytherra_core::world::{EventKind, WorldEvent};

/// How far back a broker without an Archivist's licence may read.
///
/// The same depth Mytherra's own projection grants a client without
/// `FullChronicle`, so the gate matches the world's own idea of what is recent
/// rather than inventing a second number.
const SHALLOW_CHRONICLE: usize = 32;

const PAGE: Rect = Rect {
    x: 18.0,
    y: 104.0,
    w: LOGICAL_WIDTH - 36.0,
    h: LOGICAL_HEIGHT - 122.0,
};

/// The colour a kind of event is written in. Kinds are also spelled out beside
/// each line, so this is a reinforcement and never the only signal.
fn tint(kind: EventKind) -> Color {
    match kind {
        EventKind::Divine => Color::new(0.78, 0.68, 0.95, 1.0),
        EventKind::Region => Color::new(0.62, 0.82, 0.72, 1.0),
        EventKind::Hero => Color::new(0.93, 0.82, 0.55, 1.0),
        EventKind::System => dark::TEXT_DIM,
    }
}

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    panel(PAGE, "The chronicle");

    draw_ui_text_ex(
        "the world in its own words — and what the floor did about it",
        PAGE.x + 20.0,
        PAGE.y + 48.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    draw_filters(ctx, mouse, actions);

    // What an Archivist buys is *history* — the one thing on the ladder that is
    // not an instrument class. Without that rung the chronicle is readable only
    // as far back as the world itself hands to a low-tier client, which is the
    // same depth Mytherra's own projection grants.
    let archivist = ctx.data.licences.is_archivist(ctx.session.licences().held);
    let depth = if archivist {
        usize::MAX
    } else {
        SHALLOW_CHRONICLE
    };

    let filter = ctx.session.chronicle_filter();
    let lines: Vec<&WorldEvent> = ctx
        .session
        .chronicle()
        .iter()
        .take(depth)
        .filter(|event| filter.is_none_or(|kind| event.kind == kind))
        .collect();

    if lines.is_empty() {
        draw_ui_text_ex(
            match filter {
                Some(kind) => format!("nothing of {} has been written yet", kind.label()),
                None => "the world has not turned since the exchange opened".to_owned(),
            }
            .as_str(),
            PAGE.x + 20.0,
            PAGE.y + 110.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    if !archivist {
        let withheld = ctx
            .session
            .chronicle()
            .len()
            .saturating_sub(SHALLOW_CHRONICLE);
        if withheld > 0 {
            draw_ui_text_ex(
                &format!("{withheld} older lines are archive — an Archivist's licence reads them"),
                PAGE.right() - 470.0,
                PAGE.y + 78.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
        }
    }

    // Prose on the left, and what moved on the right, sharing a year.
    let feed = Rect::new(PAGE.x + 20.0, PAGE.y + 86.0, PAGE.w - 380.0, PAGE.h - 106.0);
    draw_feed(&lines, feed);
    draw_moved(
        ctx.session.settlements(),
        Rect::new(
            feed.right() + 20.0,
            feed.y,
            PAGE.right() - feed.right() - 40.0,
            feed.h,
        ),
        mouse,
        actions,
    );
}

/// The kind filter, as chips.
fn draw_filters(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let current = ctx.session.chronicle_filter();
    let mut x = PAGE.x + 20.0;
    let y = PAGE.y + 62.0;

    for (label, kind) in [("All", None)]
        .into_iter()
        .chain(EventKind::ALL.map(|kind| (kind.label(), Some(kind))))
    {
        let width = 76.0;
        let rect = Rect::new(x, y, width, 24.0);
        let hovered = rect.contains(mouse);
        let selected = current == kind;
        let fill = if selected {
            Color::new(0.20, 0.28, 0.38, 1.0)
        } else if hovered {
            Color::new(0.16, 0.19, 0.24, 1.0)
        } else {
            Color::new(0.11, 0.12, 0.15, 1.0)
        };
        draw_surface(
            rect,
            &SurfaceStyle::new(fill).with_border(1.0, dark::ACCENT),
        );
        let text = if selected {
            dark::TEXT_BRIGHT
        } else {
            dark::TEXT_DIM
        };
        draw_ui_text_ex(
            label,
            rect.x + 10.0,
            rect.y + 16.0,
            TextStyle::new(13.0, text).params(),
        );
        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::FilterChronicle(kind));
        }
        x += width + 8.0;
    }
}

fn draw_feed(lines: &[&WorldEvent], rect: Rect) {
    let rows = (rect.h / 19.0) as usize;
    let mut y = rect.y + 14.0;
    let mut last_year = None;

    for event in lines.iter().take(rows) {
        // The year is written once per run, so a busy year reads as a block
        // rather than as the same number repeated twenty times.
        if last_year != Some(event.year) {
            draw_ui_text_ex(
                &format!("Year {}", event.year),
                rect.x,
                y,
                TextStyle::new(13.0, dark::TEXT_BRIGHT).params(),
            );
            last_year = Some(event.year);
        }
        draw_ui_text_ex(
            event.kind.label(),
            rect.x + 74.0,
            y,
            TextStyle::new(11.0, tint(event.kind)).params(),
        );
        draw_ui_text_ex(
            &truncate(&event.message, rect.w - 132.0, 13.0),
            rect.x + 132.0,
            y,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        y += 19.0;
    }

    let hidden = lines.len().saturating_sub(rows);
    if hidden > 0 {
        draw_ui_text_ex(
            &format!("{hidden} older lines held back"),
            rect.x,
            rect.bottom() - 4.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
    }
}

/// What the floor did, year by year, against the prose to its left.
///
/// Each ticker is a way through to that listing's own page — the cross-link of
/// GDD 9. It goes from a *price move* to the paper that moved, never from a line
/// of prose to a listing, because nothing here knows what the prose is about.
fn draw_moved(
    settlements: &[SettlementRecord],
    rect: Rect,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_ui_text_ex(
        "WHAT ACTUALLY MOVED",
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    if settlements.is_empty() {
        draw_ui_text_ex(
            "nothing yet",
            rect.x,
            rect.y + 38.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = rect.y + 36.0;
    for record in settlements {
        draw_ui_text_ex(
            &format!("Year {}", record.year),
            rect.x,
            y,
            TextStyle::new(13.0, dark::TEXT_BRIGHT).params(),
        );
        y += 17.0;

        if record.movers.is_empty() {
            draw_ui_text_ex(
                "  the floor did not blink",
                rect.x,
                y,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            y += 17.0;
        }
        for mover in &record.movers {
            let row = Rect::new(rect.x + 4.0, y - 12.0, rect.w - 8.0, 17.0);
            let hovered = row.contains(mouse);
            if hovered {
                draw_surface(row, &SurfaceStyle::new(Color::new(0.13, 0.16, 0.21, 1.0)));
                if is_mouse_button_released(MouseButton::Left) {
                    actions.push(UiAction::OpenListing(mover.entity.clone()));
                }
            }
            draw_ui_text_ex(
                &mover.symbol,
                rect.x + 10.0,
                y,
                TextStyle::new(
                    13.0,
                    if hovered {
                        dark::TEXT_BRIGHT
                    } else {
                        dark::TEXT
                    },
                )
                .params(),
            );
            draw_ui_text_ex(
                &signed_percent(mover.change),
                rect.x + 74.0,
                y,
                TextStyle::new(13.0, direction(mover.change)).params(),
            );
            y += 17.0;
        }
        y += 6.0;
        if y > rect.bottom() - 16.0 {
            break;
        }
    }
}
