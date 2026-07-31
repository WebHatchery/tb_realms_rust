//! The records: the equity curve, the ladder, and the seeds behind it all.
//!
//! Two things belong on this screen and nowhere else.
//!
//! **The equity curve** is the only number that says whether any of this
//! worked. Everything else on the floor is a position or an opinion; this is the
//! answer. It is drawn against the opening capital rather than against zero, so
//! "am I up" needs no arithmetic.
//!
//! **The seeds**, because a market that cannot be checked is a market a player
//! is asked to trust. The same seeds and the same orders reproduce the same
//! century exactly (GDD 5.8), and printing them is how the floor proves it is
//! not making the numbers up as it goes.

use super::widgets::{direction, draw_button, panel, signed_percent};
use super::{chart, UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::series::Series;
use macroquad_toolkit::ui::draw_ui_text_ex;

const PAGE: Rect = Rect {
    x: 18.0,
    y: 104.0,
    w: LOGICAL_WIDTH - 36.0,
    h: LOGICAL_HEIGHT - 122.0,
};

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    panel(PAGE, "Records");

    let portfolio = ctx.session.portfolio();
    let equity = portfolio.equity(ctx.session.board());
    let opening = portfolio.opening_capital;
    let ret = if opening > 0.0 {
        (equity - opening) / opening
    } else {
        0.0
    };

    draw_ui_text_ex(
        &format!("{equity:.0} crowns"),
        PAGE.x + 20.0,
        PAGE.y + 56.0,
        TextStyle::new(24.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!("{} from {opening:.0}", signed_percent(ret)),
        PAGE.x + 190.0,
        PAGE.y + 56.0,
        TextStyle::new(16.0, direction(ret)).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{} settlements observed  ·  {} trades on the tape",
            ctx.session.observed,
            portfolio.ledger().len()
        ),
        PAGE.x + 20.0,
        PAGE.y + 76.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // The curve. Drawn against a flat line at the opening capital, which is what
    // makes "up or down" readable without reading a single number.
    let plot = Rect::new(PAGE.x + 20.0, PAGE.y + 96.0, PAGE.w - 400.0, 296.0);
    let curve = ctx.session.equity_curve();
    if curve.is_empty() {
        draw_surface(
            plot,
            &SurfaceStyle::new(Color::new(0.05, 0.055, 0.07, 1.0)).with_border(1.0, dark::ACCENT),
        );
        draw_ui_text_ex(
            "no settlements yet — the curve starts when the world turns",
            plot.x + 12.0,
            plot.y + plot.h / 2.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    } else {
        // A flat series at the opening capital is the reference line, drawn
        // through the same code that draws the price against its fair value —
        // the comparison is the same shape, so it may as well be the same
        // drawing.
        let mut baseline = Series::new(curve.len().max(2));
        for _ in 0..curve.len() {
            baseline.push(opening);
        }
        chart::draw(plot, curve, &baseline, None, |value| format!("{value:.0}"));
    }

    draw_ladder(ctx, mouse, actions);
    draw_provenance(ctx, Rect::new(plot.x, plot.bottom() + 16.0, 400.0, 120.0));
    draw_honours(
        ctx,
        Rect::new(plot.x + 420.0, plot.bottom() + 16.0, plot.w - 420.0, 120.0),
    );
}

/// What the exchange has noticed. The reward is the noticing — an achievement
/// that paid out would be a second progression axis competing with the licences,
/// which are meant to be the only one (Pillar 3).
fn draw_honours(ctx: &UiContext<'_>, rect: Rect) {
    let honours = ctx.session.honours();
    let (won, all) = honours.progress();
    draw_ui_text_ex(
        &format!("WHAT THE FLOOR HAS NOTICED  ·  {won} OF {all}"),
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    // Won first, then the nearest few still open, so the column reads as a
    // record rather than as a list of things not done.
    let mut y = rect.y + 32.0;
    let ordered = honours
        .iter()
        .filter(|honour| honour.unlocked)
        .chain(honours.iter().filter(|honour| !honour.unlocked));
    for honour in ordered {
        let tint = if honour.unlocked {
            dark::TEXT_BRIGHT
        } else {
            dark::TEXT_DIM
        };
        draw_ui_text_ex(
            &format!(
                "{}  {}",
                if honour.unlocked { "[x]" } else { "[ ]" },
                super::widgets::truncate(&honour.name, 30)
            ),
            rect.x,
            y,
            TextStyle::new(13.0, tint).params(),
        );
        draw_ui_text_ex(
            &super::widgets::truncate(&honour.description, 52),
            rect.x + 230.0,
            y,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        y += 17.0;
        if y > rect.bottom() {
            break;
        }
    }
}

/// The information ladder, and the rung the broker is standing on.
fn draw_ladder(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(PAGE.right() - 360.0, PAGE.y + 96.0, 340.0, PAGE.h - 116.0);
    let table = &ctx.data.licences;
    let held = ctx.session.licences().held;

    draw_ui_text_ex(
        &format!(
            "WHAT YOU ARE ALLOWED TO SEE  ·  RUNG {} OF {}",
            held + 1,
            table.top() + 1
        ),
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    let mut y = rect.y + 36.0;
    for (index, tier) in table.tiers().iter().enumerate() {
        let owned = index <= held;
        let tint = if index == held {
            dark::TEXT_BRIGHT
        } else if owned {
            dark::TEXT
        } else {
            dark::TEXT_DIM
        };
        draw_ui_text_ex(
            &format!("{}  {}", if owned { "[x]" } else { "[ ]" }, tier.name),
            rect.x,
            y,
            TextStyle::new(15.0, tint).params(),
        );
        if !owned {
            draw_ui_text_ex(
                &format!("{:.0}", tier.cost),
                rect.right() - 60.0,
                y,
                TextStyle::new(14.0, dark::TEXT_DIM).params(),
            );
        }
        y += 17.0;
        draw_ui_text_ex(
            &format!("      {}", tier.reveals),
            rect.x,
            y,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        y += 22.0;
    }

    y += 8.0;
    match table.next(held) {
        Some(next) => {
            // The blurb of the rung *above*, because what a broker wants to know
            // is what they are missing.
            draw_ui_text_ex(
                &super::widgets::truncate(&next.blurb, 46),
                rect.x,
                y,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
            let affordable = ctx.session.portfolio().cash >= next.cost;
            if draw_button(
                Rect::new(rect.x, y + 12.0, 220.0, 32.0),
                &format!("Buy {} · {:.0}", next.name, next.cost),
                mouse,
                affordable,
            ) {
                actions.push(UiAction::BuyLicence);
            }
        }
        None => {
            draw_ui_text_ex(
                "there is nothing further to be shown",
                rect.x,
                y,
                TextStyle::new(13.0, dark::TEXT_DIM).params(),
            );
        }
    }
}

/// Where the numbers came from, so a player can check them.
fn draw_provenance(ctx: &UiContext<'_>, rect: Rect) {
    let portfolio = ctx.session.portfolio();
    draw_ui_text_ex(
        "HOW THIS CENTURY WAS MADE",
        rect.x,
        rect.y + 12.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );

    let mut y = rect.y + 34.0;
    for (label, value) in [
        ("market seed", format!("{}", ctx.data.config.market.seed)),
        (
            "noise stream now",
            format!("{:016x}", ctx.session.market_state()),
        ),
        ("fees paid", format!("{:.0}", portfolio.fees_paid)),
        (
            "coupons collected",
            format!("{:.0}", portfolio.income_received),
        ),
        ("realised", format!("{:.0}", portfolio.realised)),
    ] {
        draw_ui_text_ex(
            label,
            rect.x,
            y,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        draw_ui_text_ex(
            &value,
            rect.x + 180.0,
            y,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        y += 18.0;
    }
    draw_ui_text_ex(
        "the same seed and the same orders replay this century exactly",
        rect.x,
        y + 8.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
}
