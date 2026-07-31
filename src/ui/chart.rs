//! The price chart — the game's one piece of real visual craft (GDD 9).
//!
//! # Draw the envelope, not a line
//!
//! Each bucket of a [`Series`] knows the highest and lowest the value reached
//! inside it, so the plot is a **filled band between those two** with the
//! closing value traced over it. Once a century has been decimated into a few
//! hundred buckets, a line through the closing values alone would step neatly
//! between them and imply a calm that never happened — while the band shows
//! every spike at exactly the height it reached.
//!
//! That is the same decision `series` itself makes about what to keep, and it
//! would be wasted if the drawing then threw the extremes away. The approach is
//! taken from `dragons_hoard`, which solved it first.
//!
//! # Two series on one pair of axes
//!
//! Price is drawn against **fair value**, because the gap between them is what a
//! broker is actually trading (GDD 5.4). They share a scale and are decimated in
//! step, so the same bucket index covers the same years in both; drawing them
//! apart would be drawing two different centuries.
//!
//! # The scale includes both, and never collapses
//!
//! A range taken from the price alone would push fair value off the top the
//! moment the two diverged — which is precisely when the chart matters most.

use super::widgets::direction;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::series::Series;
use macroquad_toolkit::ui::draw_ui_text_ex;

/// The lowest and highest either series ever reached, padded so the extremes
/// are not drawn flush against the frame.
///
/// Never zero-height: a listing that has never moved would otherwise divide by
/// nothing and draw a flat line at an arbitrary place in the plot.
fn range(price: &Series, fair: &Series) -> (f32, f32) {
    let low = [price.min(), fair.min()]
        .into_iter()
        .flatten()
        .fold(f32::INFINITY, f32::min);
    let high = [price.max(), fair.max()]
        .into_iter()
        .flatten()
        .fold(f32::NEG_INFINITY, f32::max);
    if !low.is_finite() || !high.is_finite() {
        return (0.0, 1.0);
    }
    let padding = ((high - low) * 0.08).max(high.abs() * 0.02).max(0.5);
    (low - padding, high + padding)
}

/// Draw one listing's history into `plot`.
///
/// `label` formats a value for the axis, so a caller can decide how many
/// decimals a price of 3 crowns and a price of 1,400 each deserve.
pub fn draw(plot: Rect, price: &Series, fair: &Series, label: impl Fn(f32) -> String) {
    draw_surface(
        plot,
        &SurfaceStyle::new(Color::new(0.05, 0.055, 0.07, 1.0)).with_border(1.0, dark::ACCENT),
    );

    if price.is_empty() {
        draw_ui_text_ex(
            "no history yet — the world has not turned since this was listed",
            plot.x + 12.0,
            plot.y + plot.h / 2.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let (low, high) = range(price, fair);
    let span = (high - low).max(f32::EPSILON);
    let y_of = |value: f32| plot.bottom() - (value - low) / span * plot.h;

    // Gridlines, labelled. Four of them: enough to read a level off, few enough
    // that the plot is still mostly plot.
    for step in 0..=4 {
        let value = low + span * step as f32 / 4.0;
        let y = y_of(value);
        draw_line(
            plot.x,
            y,
            plot.right(),
            y,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.07),
        );
        draw_ui_text_ex(
            &label(value),
            plot.right() - 54.0,
            y - 3.0,
            TextStyle::new(11.0, dark::TEXT_DIM).params(),
        );
    }

    let buckets = price.buckets();
    let step = (plot.w - 58.0) / buckets.len().max(1) as f32;

    // The traded range, as a band.
    for (index, bucket) in buckets.iter().enumerate() {
        let x = plot.x + index as f32 * step;
        let top = y_of(bucket.max);
        let bottom = y_of(bucket.min);
        // At least a pixel: a bucket whose range is a single value still has to
        // be visible, or a flat stretch would read as missing data.
        draw_rectangle(
            x,
            top,
            step.max(1.0),
            (bottom - top).max(1.0),
            Color::new(0.55, 0.72, 0.95, 0.30),
        );
    }

    // Fair value beneath the price line, so the price reads as the foreground
    // fact and fair value as the thing it is being measured against.
    let fair_buckets = fair.buckets();
    for (index, pair) in fair_buckets.windows(2).enumerate() {
        let x = plot.x + index as f32 * step + step * 0.5;
        draw_line(
            x,
            y_of(pair[0].last),
            x + step,
            y_of(pair[1].last),
            1.2,
            Color::new(0.93, 0.78, 0.42, 0.75),
        );
    }

    // The traded price over the top.
    for (index, pair) in buckets.windows(2).enumerate() {
        let x = plot.x + index as f32 * step + step * 0.5;
        draw_line(
            x,
            y_of(pair[0].last),
            x + step,
            y_of(pair[1].last),
            1.8,
            Color::new(0.72, 0.86, 1.0, 1.0),
        );
    }

    // Where it stands now, marked on the axis it is read from.
    if let (Some(last), Some(fair_last)) = (price.last(), fair.last()) {
        let gap = last - fair_last;
        draw_ui_text_ex(
            &label(last),
            plot.right() - 54.0,
            y_of(last) - 3.0,
            TextStyle::new(12.0, direction(gap)).params(),
        );
    }

    // What one bucket covers, because a chart that has been decimated is
    // telling the truth at a coarser resolution and should say so.
    let covered = price.resolution();
    let legend = if covered > 1 {
        format!(
            "{} settlements · {covered} years per column · price against fair value",
            price.count()
        )
    } else {
        format!("{} settlements · price against fair value", price.count())
    };
    draw_ui_text_ex(
        &legend,
        plot.x + 8.0,
        plot.bottom() - 8.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(values: &[f32]) -> Series {
        let mut series = Series::new(64);
        for value in values {
            series.push(*value);
        }
        series
    }

    #[test]
    fn the_range_covers_both_series_not_just_the_price() {
        // The failure this prevents: fair value walking off the top of the plot
        // exactly when it diverges from price, which is when the chart matters.
        let price = series(&[100.0, 101.0, 99.0]);
        let fair = series(&[100.0, 260.0, 40.0]);

        let (low, high) = range(&price, &fair);

        assert!(low < 40.0, "the fair-value trough is in view: {low}");
        assert!(high > 260.0, "and so is its peak: {high}");
    }

    #[test]
    fn a_listing_that_never_moved_still_has_a_plot_to_draw_in() {
        let flat = series(&[50.0, 50.0, 50.0]);

        let (low, high) = range(&flat, &flat);

        assert!(high > low, "a zero-height range would divide by nothing");
        assert!(low < 50.0 && high > 50.0, "and the value is inside it");
    }

    #[test]
    fn an_empty_history_yields_a_usable_range_rather_than_infinities() {
        let (low, high) = range(&Series::new(8), &Series::new(8));

        assert!(low.is_finite() && high.is_finite());
        assert!(high > low);
    }

    #[test]
    fn every_bucket_lands_inside_the_plot_however_long_the_century() {
        // A century decimated many times over still has to draw inside its box.
        let mut price = Series::new(32);
        let mut fair = Series::new(32);
        for year in 0..4_000 {
            price.push(50.0 + (year % 37) as f32 * 3.0);
            fair.push(50.0 + (year % 11) as f32);
        }

        let plot = Rect::new(10.0, 20.0, 300.0, 160.0);
        let (low, high) = range(&price, &fair);
        let span = (high - low).max(f32::EPSILON);
        let y_of = |value: f32| plot.bottom() - (value - low) / span * plot.h;

        for bucket in price.buckets() {
            assert!(y_of(bucket.max) >= plot.y, "{} above the plot", bucket.max);
            assert!(
                y_of(bucket.min) <= plot.bottom(),
                "{} below the plot",
                bucket.min
            );
        }
        // And the extremes really did survive the decimation, which is the
        // whole reason `series` is the right structure here.
        assert_eq!(price.max(), Some(50.0 + 36.0 * 3.0));
        assert_eq!(price.min(), Some(50.0));
    }
}
