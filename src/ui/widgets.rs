//! The small shared pieces every panel is drawn from.
//!
//! Two of these carry a design decision rather than convenience: [`direction`]
//! is the colour-blind-safe up/down pair (GDD 12, open question 5), and
//! [`truncate`] clips on character boundaries because the chronicle's prose is
//! authored text and may not be ASCII.

use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

/// A titled surface. An empty title draws the surface alone.
pub fn panel(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);
    if !title.is_empty() {
        draw_ui_text_ex(
            title,
            rect.x + 16.0,
            rect.y + 26.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
    }
}

/// A flat button. Returns true on release inside it, so a press that drags away
/// is not a click.
pub fn draw_button(rect: Rect, label: &str, mouse: Vec2, enabled: bool) -> bool {
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
pub fn signed_percent(fraction: f32) -> String {
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
pub fn direction(change: f32) -> Color {
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
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests;
