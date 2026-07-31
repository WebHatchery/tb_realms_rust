//! One listing in full: the chart, the fundamentals it is priced on, and why it
//! last moved (GDD 9).
//!
//! This screen is where Pillar 1 is finally cashed in. Every number on it traces
//! to something: the price to a chart of every price it ever traded at, the fair
//! value to named fundamentals of a named world entity, and the last move to
//! four terms that sum to it. A player who asks "why is this worth what it is"
//! can answer the question here without leaving the page.

use super::widgets::{direction, panel, signed_percent, truncate};
use super::{chart, UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::market::Instrument;
use crate::world::fundamentals::{Fundamentals, Value};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const PAGE: Rect = Rect {
    x: 18.0,
    y: 104.0,
    w: LOGICAL_WIDTH - 36.0,
    h: LOGICAL_HEIGHT - 122.0,
};

/// How a listing's price should be written. A hamlet at 3.2 crowns and a
/// province at 1,402 do not want the same number of decimals.
fn crowns(value: f32) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    panel(PAGE, "");

    let Some(listing) = ctx.session.selected() else {
        // The world can strike a listing off while its page is open. Say so
        // rather than showing a page about nothing.
        draw_ui_text_ex(
            "that paper is no longer listed — the world stopped carrying it",
            PAGE.x + 20.0,
            PAGE.y + 60.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        if super::widgets::draw_button(
            Rect::new(PAGE.right() - 130.0, PAGE.y + 18.0, 110.0, 32.0),
            "Back",
            mouse,
            true,
        ) {
            actions.push(UiAction::CloseListing);
        }
        return;
    };

    draw_heading(ctx, listing, mouse, actions);

    let plot = Rect::new(PAGE.x + 20.0, PAGE.y + 96.0, PAGE.w - 380.0, 300.0);
    chart::draw(
        plot,
        &listing.history,
        &listing.fair_history,
        Some(&listing.era_history),
        crowns,
    );

    draw_decomposition(
        listing,
        Rect::new(plot.x, plot.bottom() + 14.0, plot.w, 128.0),
    );
    draw_fundamentals(
        ctx,
        listing,
        Rect::new(
            plot.right() + 18.0,
            PAGE.y + 96.0,
            PAGE.right() - plot.right() - 38.0,
            PAGE.bottom() - PAGE.y - 116.0,
        ),
    );
}

fn draw_heading(
    ctx: &UiContext<'_>,
    listing: &Instrument,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    draw_ui_text_ex(
        &format!("{}  {}", listing.symbol, truncate(&listing.name, 40)),
        PAGE.x + 20.0,
        PAGE.y + 34.0,
        TextStyle::new(26.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} · backed by {} · listed year {}",
            listing.class,
            listing.entity.id(),
            listing.listed_year
        ),
        PAGE.x + 20.0,
        PAGE.y + 58.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // The four numbers a broker reads first, laid out as a strip.
    let change = listing.last_move.change_fraction();
    let premium = listing.premium();
    let figures: [(&str, String, Color); 4] = [
        ("PRICE", crowns(listing.price), dark::TEXT_BRIGHT),
        ("CHANGE", signed_percent(change), direction(change)),
        ("FAIR", crowns(listing.fair_value), dark::TEXT),
        // Coloured by whether it is cheap, not by whether it is up: a listing
        // under fair value is the buy.
        ("PREMIUM", signed_percent(premium), direction(-premium)),
    ];
    let mut x = PAGE.x + 420.0;
    for (label, value, tint) in figures {
        draw_ui_text_ex(
            label,
            x,
            PAGE.y + 26.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
        draw_ui_text_ex(
            &value,
            x,
            PAGE.y + 50.0,
            TextStyle::new(20.0, tint).params(),
        );
        x += 110.0;
    }

    let held = ctx.session.portfolio().units_of(&listing.entity);
    if held > 0 {
        draw_ui_text_ex(
            &format!("holding {held}"),
            PAGE.x + 420.0,
            PAGE.y + 70.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
    }

    if super::widgets::draw_button(
        Rect::new(PAGE.right() - 130.0, PAGE.y + 18.0, 110.0, 32.0),
        "Back",
        mouse,
        true,
    ) {
        actions.push(UiAction::CloseListing);
    }
    draw_ui_text_ex(
        "or Esc",
        PAGE.right() - 122.0,
        PAGE.y + 66.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
}

/// Why the price last moved, in crowns, with the terms summing to the move.
fn draw_decomposition(listing: &Instrument, rect: Rect) {
    draw_ui_text_ex(
        "WHY IT LAST MOVED",
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    let moved = listing.last_move;
    let parts = moved.parts();
    if parts.is_empty() {
        draw_ui_text_ex(
            "it has not moved yet",
            rect.x,
            rect.y + 38.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    // A bar per term, scaled against the largest, so the shape of the move is
    // readable before any of the numbers are.
    let widest = parts
        .iter()
        .map(|(_, value)| value.abs())
        .fold(0.0_f32, f32::max)
        .max(f32::EPSILON);
    let mut y = rect.y + 32.0;
    for (name, value) in &parts {
        draw_ui_text_ex(
            name,
            rect.x,
            y + 11.0,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        let width = (value.abs() / widest) * (rect.w - 220.0);
        draw_rectangle(rect.x + 70.0, y, width.max(1.0), 12.0, direction(*value));
        draw_ui_text_ex(
            &format!("{value:+.2}"),
            rect.x + rect.w - 130.0,
            y + 11.0,
            TextStyle::new(13.0, direction(*value)).params(),
        );
        y += 20.0;
    }

    draw_ui_text_ex(
        &format!(
            "{} → {}   ({})",
            crowns(moved.from),
            crowns(moved.to),
            signed_percent(moved.change_fraction())
        ),
        rect.x,
        y + 14.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
}

/// The fundamentals the fair value above is computed from — the named world
/// numbers behind the claim.
fn draw_fundamentals(ctx: &UiContext<'_>, listing: &Instrument, rect: Rect) {
    draw_ui_text_ex(
        "WHAT IT IS A CLAIM ON",
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    let Some(entry) = ctx.session.snapshot().get(&listing.entity) else {
        draw_ui_text_ex(
            "the world has not been read yet",
            rect.x,
            rect.y + 38.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    };

    let mut y = rect.y + 36.0;
    for (field, value) in &entry.values {
        draw_ui_text_ex(
            field.key(),
            rect.x,
            y,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        let (text, tint) = match value {
            Value::Scalar(number) => (format!("{number:.1}"), dark::TEXT),
            // A flag reads as a word, not as "true" — famine is a condition,
            // not a boolean, to everyone but the pricer.
            Value::Flag(true) => ("yes".to_owned(), Color::new(0.93, 0.65, 0.30, 1.0)),
            Value::Flag(false) => ("no".to_owned(), dark::TEXT),
            Value::Status(label) => (label.clone(), dark::TEXT),
        };
        draw_ui_text_ex(
            &truncate(&text, 16),
            rect.right() - 90.0,
            y,
            TextStyle::new(13.0, tint).params(),
        );
        y += 19.0;
    }

    y += 10.0;
    draw_ui_text_ex(
        &parentage(ctx, entry),
        rect.x,
        y,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    y += 20.0;
    // What the class costs to trade, which is a fact about the paper rather
    // than about the world behind it.
    draw_ui_text_ex(
        &format!(
            "volatility {:.0}%  ·  depth {:.0}%",
            listing.volatility * 100.0,
            listing.liquidity * 100.0
        ),
        rect.x,
        y,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    if listing.income > 0.0 {
        y += 18.0;
        draw_ui_text_ex(
            &format!(
                "pays {:.2} a settlement  ·  {:.1}% yield",
                listing.income,
                listing.yield_rate() * 100.0
            ),
            rect.x,
            y,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
    }
}

/// Where this entity sits in the world, named — the link a shock travels along.
fn parentage(ctx: &UiContext<'_>, entry: &Fundamentals) -> String {
    match &entry.parent {
        Some(parent) => {
            let name = ctx
                .session
                .snapshot()
                .get(parent)
                .map(|above| above.name.clone())
                .unwrap_or_else(|| parent.id().to_owned());
            format!("within {name}")
        }
        None => "answers to no one".to_owned(),
    }
}
