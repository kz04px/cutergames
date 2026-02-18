use std::f32::consts::PI;

#[must_use]
fn erf_inv(x: f32) -> f32 {
    debug_assert!(x < 1.0);
    debug_assert!(x > -1.0);

    let a = 8.0 * (PI - 3.0) / (3.0 * PI * (4.0 - PI));
    let y = (1.0 - x * x).ln();
    let z = 2.0 / (PI * a) + y / 2.0;
    let ret = ((z * z - y / a).sqrt() - z).sqrt();
    if x >= 0.0 { ret } else { -ret }
}

#[must_use]
fn phi_inv(p: f32) -> f32 {
    2.0_f32.sqrt() * erf_inv(2.0 * p - 1.0)
}

#[must_use]
fn diff(p: f32) -> f32 {
    if p >= 1.0 {
        f32::INFINITY
    } else if p <= 0.0 {
        f32::NEG_INFINITY
    } else {
        -400.0 * (1.0 / p - 1.0).log10()
    }
}

// Normalised elo: the score's distance from 0.5 in standard deviations,
// scaled so it matches logistic elo for small differences
#[must_use]
fn ndiff(p: f32, stdev: f32) -> f32 {
    (p - 0.5) / stdev * 800.0 / 10.0_f32.ln()
}

pub mod trinomial {
    use super::{diff, ndiff, phi_inv};
    use crate::stats::WLD;

    // The variance of a single game's score around its mean
    #[must_use]
    fn variance(wld: &WLD, m_mu: f32) -> f32 {
        let dev_w = (wld.w as f32 / wld.played() as f32) * (1.0 - m_mu).powf(2.0);
        let dev_l = (wld.l as f32 / wld.played() as f32) * (0.0 - m_mu).powf(2.0);
        let dev_d = (wld.d as f32 / wld.played() as f32) * (0.5 - m_mu).powf(2.0);
        dev_w + dev_l + dev_d
    }

    #[must_use]
    pub fn elo_err(wld: &WLD) -> (f32, f32) {
        (elo(wld), err(wld))
    }

    #[must_use]
    pub fn nelo_err(wld: &WLD) -> (f32, f32) {
        (nelo(wld), nerr(wld))
    }

    #[must_use]
    pub fn elo(wld: &WLD) -> f32 {
        if wld.played() == 0 {
            f32::NAN
        } else {
            let m_mu = (wld.w as f32 + wld.d as f32 / 2.0) / wld.played() as f32;
            let n = diff(m_mu);
            if n == -0.0 { 0.0 } else { n }
        }
    }

    #[must_use]
    pub fn err(wld: &WLD) -> f32 {
        if wld.played() == 0 {
            f32::NAN
        } else {
            let m_mu = wld.winrate();
            let m_stdev = variance(wld, m_mu).sqrt() / (wld.played() as f32).sqrt();
            let mu_min = m_mu + phi_inv(0.025) * m_stdev;
            let mu_max = m_mu + phi_inv(0.975) * m_stdev;
            (diff(mu_max) - diff(mu_min)) / 2.0
        }
    }

    #[must_use]
    pub fn nelo(wld: &WLD) -> f32 {
        if wld.played() == 0 {
            f32::NAN
        } else {
            let m_mu = wld.winrate();
            ndiff(m_mu, variance(wld, m_mu).sqrt())
        }
    }

    #[must_use]
    pub fn nerr(wld: &WLD) -> f32 {
        if wld.played() == 0 {
            f32::NAN
        } else {
            let m_mu = wld.winrate();
            let stdev = variance(wld, m_mu).sqrt();
            let m_stdev = stdev / (wld.played() as f32).sqrt();
            let mu_min = m_mu + phi_inv(0.025) * m_stdev;
            let mu_max = m_mu + phi_inv(0.975) * m_stdev;
            (ndiff(mu_max, stdev) - ndiff(mu_min, stdev)) / 2.0
        }
    }
}

pub mod pentanomial {
    use super::{diff, ndiff, phi_inv};
    use crate::stats::WLDPairs;

    // A game pair's total score (0, 0.5, 1, 1.5 or 2) bucketed by result,
    // ignoring which player was first to move.
    struct Buckets {
        n0: f32,
        n05: f32,
        n1: f32,
        n15: f32,
        n2: f32,
        pairs: f32,
    }

    #[must_use]
    fn buckets(pairs: &WLDPairs) -> Buckets {
        Buckets {
            n0: pairs.ll as f32,
            n05: (pairs.dl + pairs.ld) as f32,
            n1: (pairs.wl + pairs.lw + pairs.dd) as f32,
            n15: (pairs.wd + pairs.dw) as f32,
            n2: pairs.ww as f32,
            pairs: (pairs.played() / 2) as f32,
        }
    }

    // The variance of a single pair's normalised score around its mean
    #[must_use]
    fn variance(b: &Buckets, m_mu: f32) -> f32 {
        let dev0 = (b.n0 / b.pairs) * (0.0 - m_mu).powf(2.0);
        let dev05 = (b.n05 / b.pairs) * (0.25 - m_mu).powf(2.0);
        let dev1 = (b.n1 / b.pairs) * (0.5 - m_mu).powf(2.0);
        let dev15 = (b.n15 / b.pairs) * (0.75 - m_mu).powf(2.0);
        let dev2 = (b.n2 / b.pairs) * (1.0 - m_mu).powf(2.0);
        dev0 + dev05 + dev1 + dev15 + dev2
    }

    #[must_use]
    pub fn elo_err(pairs: &WLDPairs) -> (f32, f32) {
        (elo(pairs), err(pairs))
    }

    #[must_use]
    pub fn nelo_err(pairs: &WLDPairs) -> (f32, f32) {
        (nelo(pairs), nerr(pairs))
    }

    #[must_use]
    pub fn elo(pairs: &WLDPairs) -> f32 {
        let b = buckets(pairs);
        if b.pairs == 0.0 {
            f32::NAN
        } else {
            // Normalised mean score in [0, 1], a pair's max score being 2
            let m_mu = pairs.winrate();
            let n = diff(m_mu);
            if n == -0.0 { 0.0 } else { n }
        }
    }

    #[must_use]
    pub fn err(pairs: &WLDPairs) -> f32 {
        let b = buckets(pairs);
        if b.pairs == 0.0 {
            f32::NAN
        } else {
            let m_mu = pairs.winrate();
            let m_stdev = variance(&b, m_mu).sqrt() / b.pairs.sqrt();
            let mu_min = m_mu + phi_inv(0.025) * m_stdev;
            let mu_max = m_mu + phi_inv(0.975) * m_stdev;
            (diff(mu_max) - diff(mu_min)) / 2.0
        }
    }

    // A pair is two games, so its normalised score's deviation is scaled by
    // sqrt(2) to give the per-game figure that nElo is defined on
    #[must_use]
    pub fn nelo(pairs: &WLDPairs) -> f32 {
        let b = buckets(pairs);
        if b.pairs == 0.0 {
            f32::NAN
        } else {
            let m_mu = pairs.winrate();
            ndiff(m_mu, (2.0 * variance(&b, m_mu)).sqrt())
        }
    }

    #[must_use]
    pub fn nerr(pairs: &WLDPairs) -> f32 {
        let b = buckets(pairs);
        if b.pairs == 0.0 {
            f32::NAN
        } else {
            let m_mu = pairs.winrate();
            let var = variance(&b, m_mu);
            let stdev = (2.0 * var).sqrt();
            let m_stdev = var.sqrt() / b.pairs.sqrt();
            let mu_min = m_mu + phi_inv(0.025) * m_stdev;
            let mu_max = m_mu + phi_inv(0.975) * m_stdev;
            (ndiff(mu_max, stdev) - ndiff(mu_min, stdev)) / 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{WLD, WLDPairs};

    #[test]
    fn trinomial_elo() {
        let tests = [
            ((1, 0, 0), f32::INFINITY),
            ((0, 1, 0), f32::NEG_INFINITY),
            ((0, 0, 1), 0.0),
            ((7, 3, 0), 147.2),
            ((12, 6, 2), 107.5),
            ((20, 8, 2), 147.2),
            ((25, 11, 4), 127.0),
            ((29, 14, 7), 107.5),
            ((36, 15, 9), 127.0),
            ((41, 17, 12), 124.1),
            ((47, 18, 15), 131.9),
            ((53, 21, 16), 129.2),
            ((55, 26, 19), 103.7),
        ];

        for ((w, l, d), elo) in tests {
            let got = trinomial::elo(&WLD { w, l, d });
            let diff = (got - elo).abs();
            assert!(got == elo || diff <= 0.1);
        }
    }

    #[test]
    fn trinomial_err() {
        let tests = [
            ((6, 3, 1), 268.4),
            ((12, 6, 2), 165.0),
            ((20, 7, 3), 140.1),
            ((25, 9, 6), 111.8),
            ((31, 9, 10), 96.7),
            ((39, 11, 10), 91.4),
            ((43, 15, 12), 81.3),
            ((48, 18, 14), 74.8),
            ((54, 21, 15), 70.6),
            ((60, 22, 18), 66.5),
        ];

        for ((w, l, d), err) in tests {
            let got = trinomial::err(&WLD { w, l, d });
            let diff = (got - err).abs();
            assert!(diff <= 0.1);
        }
    }

    #[test]
    fn pentanomial_no_games() {
        let pairs = WLDPairs::default();
        assert!(pentanomial::elo(&pairs).is_nan());
        assert!(pentanomial::err(&pairs).is_nan());
    }

    #[test]
    fn pentanomial_elo() {
        let tests = [
            // A single pair with this outcome, expected elo
            (
                WLDPairs {
                    ww: 1,
                    ..Default::default()
                },
                f32::INFINITY,
            ),
            (
                WLDPairs {
                    ll: 1,
                    ..Default::default()
                },
                f32::NEG_INFINITY,
            ),
            (
                WLDPairs {
                    dd: 1,
                    ..Default::default()
                },
                0.0,
            ),
            (
                WLDPairs {
                    wl: 1,
                    lw: 1,
                    ..Default::default()
                },
                0.0,
            ),
            (
                WLDPairs {
                    wd: 1,
                    ..Default::default()
                },
                190.8,
            ),
            (
                WLDPairs {
                    ww: 2,
                    wd: 3,
                    dd: 3,
                    ld: 1,
                    ll: 1,
                    ..Default::default()
                },
                70.4,
            ),
        ];

        for (pairs, expected) in tests {
            let got = pentanomial::elo(&pairs);
            let diff = (got - expected).abs();
            assert!(got == expected || diff <= 0.2);
        }
    }

    #[test]
    fn pentanomial_err() {
        // A single pair off the 0%/100% extremes has no spread to measure,
        // so the error is zero
        assert_eq!(
            pentanomial::err(&WLDPairs {
                wd: 1,
                ..Default::default()
            }),
            0.0
        );

        // At the 0%/100% extremes both confidence bounds diverge to
        // infinity, so the width between them is undefined (NaN) -- the
        // same behaviour as trinomial::err in that limit
        assert!(
            pentanomial::err(&WLDPairs {
                ww: 1,
                ..Default::default()
            })
            .is_nan()
        );
        assert!(
            pentanomial::err(&WLDPairs {
                ll: 1,
                ..Default::default()
            })
            .is_nan()
        );

        // A mixed sample has genuine spread
        let mixed = WLDPairs {
            ww: 2,
            wd: 3,
            dd: 3,
            ld: 1,
            ll: 1,
            ..Default::default()
        };
        let got = pentanomial::err(&mixed);
        assert!(got.is_finite());
        assert!((got - 143.1).abs() <= 5.0);
    }

    #[test]
    fn trinomial_nelo() {
        // (w, l, d), expected nelo, expected error
        let tests = [
            ((7, 3, 0), 151.6, 215.3),
            ((55, 26, 19), 118.3, 68.1),
            ((116, 23, 269), 147.4, 33.7),
        ];

        for ((w, l, d), nelo, nerr) in tests {
            let (got_nelo, got_nerr) = trinomial::nelo_err(&WLD { w, l, d });
            assert!((got_nelo - nelo).abs() <= 0.1);
            assert!((got_nerr - nerr).abs() <= 0.005 * nerr);
        }

        assert!(trinomial::nelo(&WLD::default()).is_nan());
        assert!(trinomial::nerr(&WLD::default()).is_nan());
    }

    #[test]
    fn pentanomial_nelo() {
        assert!(pentanomial::nelo(&WLDPairs::default()).is_nan());
        assert!(pentanomial::nerr(&WLDPairs::default()).is_nan());

        let mixed = WLDPairs {
            ww: 2,
            wd: 3,
            dd: 3,
            ld: 1,
            ll: 1,
            ..Default::default()
        };
        let (got_nelo, got_nerr) = pentanomial::nelo_err(&mixed);
        assert!((got_nelo - 81.9).abs() <= 0.1);
        assert!((got_nerr - 152.3).abs() <= 0.005 * 152.3);
    }
}
