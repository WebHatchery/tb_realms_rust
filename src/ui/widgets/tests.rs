use super::*;

#[test]
fn a_move_reads_the_same_without_colour_as_with_it() {
    // The accessibility claim, asserted: the sign carries the direction on
    // its own, so a player who cannot separate the two hues loses nothing.
    assert_eq!(signed_percent(0.042), "+4.2%");
    assert_eq!(signed_percent(-0.042), "-4.2%");
    assert_eq!(signed_percent(0.0), "—");
    assert_ne!(direction(0.1), direction(-0.1));
}
