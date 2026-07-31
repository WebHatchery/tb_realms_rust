//! The research browser: every entity the exchange watches, class by class.
//!
//! This is the world seen directly rather than through a price — the same
//! reading the pricer works from, shown raw. Two things make it worth a screen
//! of its own:
//!
//! - It includes entities that are **not listed**. A resource node is supply
//!   behind a commodity and can never be held, but a broker who wants to know
//!   why grain is dear should be able to count the nodes still producing.
//! - It is the entry point to a listing's page, so research leads to a position
//!   rather than to a dead end.
//!
//! Under GDD 5.3 this screen is what an information licence eventually gates.
//! Nothing is gated yet — licences are M5 — so it currently shows the whole
//! world, which is what a spectator's Standing reveals.

use super::widgets::{panel, signed_percent, truncate};
use super::{UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::world::fundamentals::{Field, Fundamentals, Value};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

const PAGE: Rect = Rect {
    x: 18.0,
    y: 104.0,
    w: LOGICAL_WIDTH - 36.0,
    h: LOGICAL_HEIGHT - 122.0,
};

/// The classes a broker browses by, and the one field worth putting in a
/// column beside each name. `None` for a class whose interest is its status
/// rather than any number.
const CLASSES: [(&str, &str, Option<Field>); 8] = [
    ("Provinces", "regional_bond", Some(Field::Prosperity)),
    ("Towns", "settlement_share", Some(Field::Population)),
    ("Houses", "house_scrip", Some(Field::LivingRenown)),
    ("Orders", "order_charter", Some(Field::LivingMembers)),
    ("Roads", "route_toll", Some(Field::Volume)),
    ("Commodities", "commodity", Some(Field::NodesProducing)),
    ("Relics", "relic_lot", Some(Field::Power)),
    // Not a listing class: nodes are supply, and this is the only screen that
    // shows them at all.
    ("Nodes", "resource_node", None),
];

pub fn draw(ctx: &UiContext<'_>, mouse: Vec2, actions: &mut Vec<UiAction>) {
    panel(PAGE, "Research");

    draw_ui_text_ex(
        "the world as the pricer reads it — every number a fair value is built from",
        PAGE.x + 20.0,
        PAGE.y + 48.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let chosen = ctx.session.research_class();
    draw_classes(ctx, chosen, mouse, actions);

    // The entities of the chosen class, as the last reading of the world has
    // them. `resource_node` is the one key with no listing class behind it.
    let mut rows: Vec<&Fundamentals> = ctx
        .session
        .snapshot()
        .iter()
        .filter(|entry| match entry.entity.listing_class() {
            Some(class) => class == chosen,
            None => chosen == "resource_node",
        })
        .collect();

    let column = CLASSES
        .iter()
        .find(|(_, key, _)| *key == chosen)
        .and_then(|(_, _, field)| *field);
    // Ranked by the column shown, so the screen leads with what matters in that
    // class rather than with whatever the world happened to list first.
    if let Some(field) = column {
        rows.sort_by(|a, b| {
            b.scalar(field)
                .unwrap_or(0.0)
                .total_cmp(&a.scalar(field).unwrap_or(0.0))
        });
    }

    draw_rows(ctx, &rows, column, mouse, actions);
}

fn draw_classes(ctx: &UiContext<'_>, chosen: &str, mouse: Vec2, actions: &mut Vec<UiAction>) {
    let mut y = PAGE.y + 78.0;
    for (label, key, _) in CLASSES {
        let rect = Rect::new(PAGE.x + 20.0, y, 150.0, 28.0);
        let hovered = rect.contains(mouse);
        let selected = chosen == key;
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

        let count = ctx
            .session
            .snapshot()
            .iter()
            .filter(|entry| match entry.entity.listing_class() {
                Some(class) => class == key,
                None => key == "resource_node",
            })
            .count();
        draw_ui_text_ex(
            label,
            rect.x + 10.0,
            rect.y + 19.0,
            TextStyle::new(
                14.0,
                if selected {
                    dark::TEXT_BRIGHT
                } else {
                    dark::TEXT
                },
            )
            .params(),
        );
        draw_ui_text_ex(
            &count.to_string(),
            rect.right() - 34.0,
            rect.y + 19.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::Research(key.to_owned()));
        }
        y += 32.0;
    }
}

fn draw_rows(
    ctx: &UiContext<'_>,
    rows: &[&Fundamentals],
    column: Option<Field>,
    mouse: Vec2,
    actions: &mut Vec<UiAction>,
) {
    let table = Rect::new(PAGE.x + 186.0, PAGE.y + 78.0, PAGE.w - 206.0, PAGE.h - 98.0);

    if rows.is_empty() {
        draw_ui_text_ex(
            "the world carries none of these",
            table.x,
            table.y + 24.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let value_x = table.x + 320.0;
    let price_x = table.x + 440.0;
    let premium_x = table.x + 540.0;
    draw_ui_text_ex(
        "NAME",
        table.x + 6.0,
        table.y + 14.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
    if let Some(field) = column {
        draw_ui_text_ex(
            &field.key().to_uppercase(),
            value_x,
            table.y + 14.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }
    for (label, x) in [("PRICE", price_x), ("PREM", premium_x)] {
        draw_ui_text_ex(
            label,
            x,
            table.y + 14.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }

    let capacity = ((table.h - 30.0) / 21.0) as usize;
    let mut y = table.y + 34.0;
    for entry in rows.iter().take(capacity) {
        let listed = ctx.session.board().get(&entry.entity);
        let row = Rect::new(table.x, y - 14.0, table.w, 21.0);
        let hovered = row.contains(mouse) && listed.is_some();
        if hovered {
            draw_surface(row, &SurfaceStyle::new(Color::new(0.13, 0.16, 0.21, 1.0)));
            if is_mouse_button_released(MouseButton::Left) {
                actions.push(UiAction::OpenListing(entry.entity.clone()));
            }
        }

        draw_ui_text_ex(
            &truncate(&entry.name, 34),
            table.x + 6.0,
            y,
            TextStyle::new(14.0, dark::TEXT).params(),
        );
        if let Some(field) = column {
            draw_ui_text_ex(
                &reading(entry, field),
                value_x,
                y,
                TextStyle::new(13.0, dark::TEXT).params(),
            );
        }

        match listed {
            Some(listing) => {
                draw_ui_text_ex(
                    &format!("{:.1}", listing.price),
                    price_x,
                    y,
                    TextStyle::new(13.0, dark::TEXT_BRIGHT).params(),
                );
                draw_ui_text_ex(
                    &signed_percent(listing.premium()),
                    premium_x,
                    y,
                    TextStyle::new(13.0, super::widgets::direction(-listing.premium())).params(),
                );
            }
            // A node is watched but never listed, and saying so is the point of
            // showing it here at all.
            None => {
                draw_ui_text_ex(
                    "not listed",
                    price_x,
                    y,
                    TextStyle::new(12.0, dark::TEXT_DIM).params(),
                );
            }
        }
        y += 21.0;
    }

    let hidden = rows.len().saturating_sub(capacity);
    let footer = if hidden > 0 {
        format!("{} entities · {hidden} below the fold", rows.len())
    } else {
        format!("{} entities", rows.len())
    };
    draw_ui_text_ex(
        &footer,
        table.x + 6.0,
        table.bottom() - 4.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
}

/// One field of one entity, written the way that field deserves.
fn reading(entry: &Fundamentals, field: Field) -> String {
    match entry.values.get(&field) {
        Some(Value::Scalar(value)) => format!("{value:.1}"),
        Some(Value::Flag(true)) => "yes".to_owned(),
        Some(Value::Flag(false)) => "no".to_owned(),
        Some(Value::Status(label)) => label.clone(),
        None => "—".to_owned(),
    }
}
