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
