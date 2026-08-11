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
