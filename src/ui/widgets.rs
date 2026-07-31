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
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_lines_and_clips_long_ones() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 10), "abcdefghij");
        assert_eq!(truncate("abcdefghijk", 10), "abcdefghi…");
    }

    #[test]
    fn truncate_splits_on_character_boundaries() {
        // The chronicle is authored prose and may carry non-ASCII; slicing by
        // byte would panic here.
        let line = "æther — the Arcane Circle rises in Duskvale";
        let clipped = truncate(line, 12);
        assert_eq!(clipped.chars().count(), 12);
        assert!(clipped.ends_with('…'));
    }

    #[test]
    fn a_move_reads_the_same_without_colour_as_with_it() {
        // The accessibility claim, asserted: the sign carries the direction on
        // its own, so a player who cannot separate the two hues loses nothing.
        assert_eq!(signed_percent(0.042), "+4.2%");
        assert_eq!(signed_percent(-0.042), "-4.2%");
        assert_eq!(signed_percent(0.0), "—");
        assert_ne!(direction(0.1), direction(-0.1));
    }
}
