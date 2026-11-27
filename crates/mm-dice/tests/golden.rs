//! Step-1 golden (architecture §11.1): research 03 §7.2's level-5 greatsword round,
//! built from dice alone.
//!
//! The attacker: +7 to hit (Str +4, PB +3) against AC 16, two attacks with a 2d6 Heavy
//! weapon whose misses deal Graze damage equal to the Str modifier (4), a fighting style
//! that counts 1s and 2s as 3, +PB damage on hits made as part of the Attack action, and a
//! Bonus Action attack (Hew) if either attack crits. Hew gets no +PB in the base reading.
//!
//! The published figures are rounded, so each is checked to within half a unit of its
//! last stated digit.

use mm_dice::{AcLanes, AttackRoll, Pmf, RollMode};
use mm_testkit::tolerance::{assert_close, assert_rounds_to};

const STR: i32 = 4;
const PB: i32 = 3;

/// Whether the crit-gated Bonus Action attack is taken, and whether it gets +PB.
#[derive(Clone, Copy)]
enum Hew {
    Off,
    WithoutPb,
    WithPb,
}

/// The parts of the round a variant switches on or off.
#[derive(Clone, Copy)]
struct Round {
    hew: Hew,
    graze: bool,
    style: bool,
}

const BASE: Round = Round {
    hew: Hew::WithoutPb,
    graze: true,
    style: true,
};

/// One attack split into its non-crit and crit branches (partial distributions).
struct Attack {
    no_crit: Pmf,
    crit: Pmf,
}

impl Attack {
    /// Both branches together.
    fn whole(&self) -> Pmf {
        Pmf::mix(&[(1.0, &self.no_crit), (1.0, &self.crit)])
    }
}

impl Round {
    fn weapon_die(self) -> Pmf {
        if self.style {
            Pmf::die(6).at_least(3)
        } else {
            Pmf::die(6)
        }
    }

    fn attack(self, ac: i32, flat: i32) -> Attack {
        let odds = AttackRoll {
            bonus: STR + PB,
            ac,
            crit_on: 20,
            mode: RollMode::Normal,
        }
        .odds();
        let hit = self.weapon_die().sum_of(2).shift(flat);
        let crit = self.weapon_die().sum_of(4).shift(flat);
        let miss = Pmf::point(if self.graze { STR } else { 0 });
        Attack {
            no_crit: Pmf::mix(&[(odds.hit, &hit), (odds.miss, &miss)]),
            crit: Pmf::mix(&[(odds.crit, &crit)]),
        }
    }

    fn damage(self, ac: i32) -> Pmf {
        let main = self.attack(ac, STR + PB);
        let no_crit = main.no_crit.convolve(&main.no_crit);
        // At least one crit: the first attack crits, or the first does not and the second does.
        let some_crit = Pmf::mix(&[
            (1.0, &main.crit.convolve(&main.whole())),
            (1.0, &main.no_crit.convolve(&main.crit)),
        ]);
        let hew_flat = match self.hew {
            Hew::Off => None,
            Hew::WithoutPb => Some(STR),
            Hew::WithPb => Some(STR + PB),
        };
        let after_crit = match hew_flat {
            Some(flat) => {
                let hew = self.attack(ac, flat);
                some_crit.convolve(&hew.whole())
            }
            None => some_crit,
        };
        Pmf::mix(&[(1.0, &no_crit), (1.0, &after_crit)])
    }
}

#[test]
fn base_round_matches_research_03_worked_example() {
    let d = BASE.damage(16);
    assert_close(d.mass(), 1.0);
    assert_rounds_to("mean", d.mean(), 22.897, 0.001);
    assert_rounds_to("SD", d.variance().sqrt(), 10.26, 0.01);
    assert_eq!(d.support(), (8, 90), "min and max");
    assert_eq!(d.quantile(0.10), 8, "p10");
    assert_eq!(d.quantile(0.25), 17, "p25");
    assert_eq!(d.quantile(0.50), 20, "median");
    assert_eq!(d.quantile(0.75), 30, "p75");
    assert_eq!(d.quantile(0.90), 34, "p90");
    assert_eq!(d.quantile(0.95), 42, "p95");
    // Both attacks graze: exactly 8 with probability 0.40².
    assert_close(d.prob(8), 0.16);
    for (at_least, p) in [
        (10, 0.840),
        (20, 0.571),
        (30, 0.261),
        (40, 0.064),
        (50, 0.022),
    ] {
        assert_rounds_to(&format!("P(>= {at_least})"), d.tail(at_least), p, 0.001);
    }
}

#[test]
fn variant_table_matches_research_03() {
    let variants = [
        (
            "no Hew",
            Round {
                hew: Hew::Off,
                ..BASE
            },
            22.00,
            8.64,
        ),
        (
            "no Graze",
            Round {
                graze: false,
                ..BASE
            },
            19.54,
            12.49,
        ),
        (
            "no style",
            Round {
                style: false,
                ..BASE
            },
            21.53,
            9.57,
        ),
        (
            "Hew with +PB",
            Round {
                hew: Hew::WithPb,
                ..BASE
            },
            23.07,
            10.68,
        ),
    ];
    for (name, round, mean, sd) in variants {
        let d = round.damage(16);
        assert_rounds_to(&format!("{name} mean"), d.mean(), mean, 0.01);
        assert_rounds_to(&format!("{name} SD"), d.variance().sqrt(), sd, 0.01);
    }
}

#[test]
fn ac_sensitivity_matches_research_03() {
    // Research 03 §7.2, "AC sensitivity": the base round's mean, to the nearest 0.1. The
    // note gives 12.7 at AC 25, but by hand (+7 needs an 18: miss 0.85, hit 0.10, crit 0.05)
    // the round is 2·(0.10·15 + 0.05·23 + 0.85·4) + 0.0975·(0.10·12 + 0.05·20 + 0.85·4)
    // = 12.10 + 0.546 = 12.646, which rounds to 12.6.
    let means = AcLanes::from_fn(|ac| BASE.damage(ac).mean());
    for (ac, mean) in [
        (10, 29.7),
        (13, 26.3),
        (16, 22.9),
        (19, 19.5),
        (22, 16.1),
        (25, 12.6),
    ] {
        assert_rounds_to(&format!("mean at AC {ac}"), means[ac], mean, 0.1);
    }
}
