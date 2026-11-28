//! Damage distributions on dice small enough to count by hand.

use mm_dice::Pmf;
use mm_testkit::tolerance::assert_close;

fn two_d6() -> Pmf {
    Pmf::die(6).sum_of(2)
}

#[test]
fn a_single_die_is_uniform_over_its_faces() {
    let d8 = Pmf::die(8);
    assert_eq!(d8.support(), (1, 8));
    for face in 1..=8 {
        assert_close(d8.prob(face), 0.125);
    }
    assert_close(d8.mean(), 4.5);
    assert_close(d8.variance(), 63.0 / 12.0);
}

#[test]
fn two_d6_has_the_triangular_distribution() {
    let d = two_d6();
    assert_eq!(d.support(), (2, 12));
    assert_close(d.prob(7), 6.0 / 36.0);
    assert_close(d.prob(2), 1.0 / 36.0);
    assert_close(d.mass(), 1.0);
    assert_close(d.mean(), 7.0);
    assert_close(d.variance(), 35.0 / 6.0);
    assert_close(d.tail(11), 3.0 / 36.0);
    assert_eq!(d.quantile(0.5), 7);
    // 21/36 of the mass is at or below 7, 15/36 at or below 6.
    assert_eq!(d.quantile(15.0 / 36.0 + 0.01), 7);
    assert_eq!(d.quantile(0.0), 2);
    assert_eq!(d.quantile(1.0), 12);
}

#[test]
fn counting_low_faces_as_three_gives_a_d6_mean_of_four() {
    // Research 03 §7.2: "1s and 2s count as 3, so E[d6] = 4".
    let d = Pmf::die(6).at_least(3);
    assert_eq!(d.support(), (3, 6));
    assert_close(d.prob(3), 0.5);
    assert_close(d.mean(), 4.0);
}

#[test]
fn shift_adds_a_flat_modifier() {
    let d = two_d6().shift(7);
    assert_eq!(d.support(), (9, 19));
    assert_close(d.mean(), 14.0);
    assert_close(d.variance(), 35.0 / 6.0);
}

#[test]
fn at_most_piles_the_excess_on_the_cap() {
    let d = two_d6().at_most(10);
    assert_eq!(d.support(), (2, 10));
    assert_close(d.prob(10), 6.0 / 36.0);
    assert_close(d.mass(), 1.0);
}

#[test]
fn convolving_independent_dice_adds_them() {
    let d = Pmf::die(4).convolve(&Pmf::die(6));
    assert_eq!(d.support(), (2, 10));
    assert_close(d.prob(5), 4.0 / 24.0);
    assert_close(d.mean(), 6.0);
}

#[test]
fn mix_weights_each_branch() {
    // A quarter of the time 10, otherwise a d4.
    let d = Pmf::mix(&[(0.25, &Pmf::point(10)), (0.75, &Pmf::die(4))]);
    assert_eq!(d.support(), (1, 10));
    assert_close(d.prob(10), 0.25);
    assert_close(d.prob(3), 0.1875);
    assert_close(d.mean(), 0.25 * 10.0 + 0.75 * 2.5);
    assert_close(d.tail(5), 0.25);
}

#[test]
fn branches_of_a_mix_may_be_partial() {
    // Mixing partial distributions with weight 1 adds them: the miss branch (0.4 at 0)
    // and the hit branch (0.6 spread over a d6) together make one attack.
    let miss = Pmf::mix(&[(0.4, &Pmf::point(0))]);
    let hit = Pmf::mix(&[(0.6, &Pmf::die(6))]);
    assert_close(miss.mass(), 0.4);
    let attack = Pmf::mix(&[(1.0, &miss), (1.0, &hit)]);
    assert_close(attack.mass(), 1.0);
    assert_close(attack.mean(), 0.6 * 3.5);
}

#[test]
fn the_mean_of_the_lowest_tenth_takes_part_of_the_bin_that_crosses_it() {
    // 2d6: 2 has 1/36, 3 has 2/36, 4 has 3/36. The lowest 0.1 = 3.6/36 of the mass is
    // all of 2 and 3 and 0.6/36 of the 4s: (2·1 + 3·2 + 4·0.6) / 3.6 = 10.4 / 3.6 = 26/9.
    assert_close(two_d6().mean_of_lowest(0.1), 26.0 / 9.0);
    // All of the mass is the whole mean.
    assert_close(two_d6().mean_of_lowest(1.0), 7.0);
    // A certain total is its own floor.
    assert_close(Pmf::point(8).mean_of_lowest(0.1), 8.0);
}

#[test]
fn the_tail_beyond_the_support_is_positive_zero() {
    // Not −0.0, the empty sum of `Iterator::sum`, which would print as "-0.0".
    assert_eq!(two_d6().tail(13).to_bits(), 0.0_f64.to_bits());
}
