//! The ledger: what the broker holds, what it cost, and what it is worth.
//!
//! Fees are shown as a line of their own rather than folded quietly into the
//! basis figures above them. They *are* in the basis — a P&L that ignores what
//! the trading cost is a P&L that flatters the trader — but a broker should be
//! able to see how much of their century went to the floor.

use super::widgets::{direction, panel, signed_percent};
use super::{UiContext, LEDGER};
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(ctx: &UiContext<'_>) {
    let rect = LEDGER;
    panel(rect, "The ledger");

    let portfolio = ctx.session.portfolio();
    let board = ctx.session.board();
    let equity = portfolio.equity(board);
    let ret = if portfolio.opening_capital > 0.0 {
        (equity - portfolio.opening_capital) / portfolio.opening_capital
    } else {
        0.0
    };

    draw_ui_text_ex(
        &format!("{equity:.0} crowns"),
        rect.x + 16.0,
        rect.y + 58.0,
        TextStyle::new(22.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &signed_percent(ret),
        rect.right() - 78.0,
        rect.y + 58.0,
        TextStyle::new(18.0, direction(ret)).params(),
    );

    let mut y = rect.y + 84.0;
    for (label, value) in [
        ("cash", portfolio.cash),
        ("holdings", portfolio.holdings_value(board)),
        ("unrealised", portfolio.unrealised(board)),
        ("realised", portfolio.realised),
        ("coupons", portfolio.income_received),
        ("fees paid", -portfolio.fees_paid),
    ] {
        draw_ui_text_ex(
            label,
            rect.x + 16.0,
            y,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        // `+ 0.0` normalises negative zero, which would otherwise print as
        // "-0" on every line the broker has not yet touched.
        draw_ui_text_ex(
            &format!("{:.0}", value + 0.0),
            rect.right() - 78.0,
            y,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        y += 17.0;
    }

    y += 14.0;
    draw_ui_text_ex(
        "POSITIONS",
        rect.x + 16.0,
        y,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    y += 20.0;

    let mut positions: Vec<_> = portfolio.positions().collect();
    if positions.is_empty() {
        draw_ui_text_ex(
            "nothing held — pick a listing and place a ticket",
            rect.x + 16.0,
            y,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    }
    positions.sort_by(|a, b| b.cost_basis.total_cmp(&a.cost_basis));

    for position in positions {
        // A position whose listing is gone is written off at the next
        // settlement; until then it simply has nothing to price against.
        let Some(listing) = board.get(&position.entity) else {
            continue;
        };
        draw_ui_text_ex(
            &format!("{} ×{}", position.symbol, position.units),
            rect.x + 16.0,
            y,
            TextStyle::new(14.0, dark::TEXT_BRIGHT).params(),
        );
        let profit = position.unrealised(listing.price);
        draw_ui_text_ex(
            &format!("{profit:+.0}"),
            rect.right() - 78.0,
            y,
            TextStyle::new(14.0, direction(profit)).params(),
        );
        y += 16.0;
        draw_ui_text_ex(
            &format!(
                "  cost {:.1}  ·  now {:.1}",
                position.average_cost(),
                listing.price
            ),
            rect.x + 16.0,
            y,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        y += 21.0;
        if y > rect.bottom() - 16.0 {
            break;
        }
    }
}
