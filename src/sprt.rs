use crate::stats::{WLD, WLDPairs};

// The SPRT here follows fastchess (https://github.com/Disservin/fastchess), so
// the same results give the same LLR. The three models differ in what the
// elo bounds mean:
//   Normalized  elo0 and elo1 are normalised elo (nElo), the default
//   Logistic    elo0 and elo1 are ordinary (logistic) elo
//   Bayesian    elo0 and elo1 are BayesElo; win/draw/loss results only
// The maths is from Michel Van den Bergh's notes behind Fishtest:
//   https://www.cantate.be/Fishtest/support_MLE_multinomial.pdf
//   https://www.cantate.be/Fishtest/normalized_elo_practical.pdf
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Model {
    #[default]
    Normalized,
    Logistic,
    Bayesian,
}

impl Model {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "normalized" => Some(Self::Normalized),
            "logistic" => Some(Self::Logistic),
            "bayesian" => Some(Self::Bayesian),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Normalized => "normalized",
            Self::Logistic => "logistic",
            Self::Bayesian => "bayesian",
        }
    }
}

// Normalised elo is divided by this to get the t-value the test works with
fn nelo_divided_by_nt() -> f64 {
    800.0 / 10.0_f64.ln()
}

fn logistic_elo_to_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf(-elo / 400.0))
}

fn bayeselo_to_score(bayeselo: f64, drawelo: f64) -> f64 {
    let pwin = 1.0 / (1.0 + 10.0_f64.powf((-bayeselo + drawelo) / 400.0));
    let ploss = 1.0 / (1.0 + 10.0_f64.powf((bayeselo + drawelo) / 400.0));
    let pdraw = 1.0 - pwin - ploss;
    pwin + 0.5 * pdraw
}

// A count of zero is replaced by a tiny one, so no outcome has probability
// zero.
fn regularise(count: usize) -> f64 {
    if count == 0 { 1e-3 } else { count as f64 }
}

fn mean<const N: usize>(x: &[f64; N], p: &[f64; N]) -> f64 {
    (0..N).map(|i| x[i] * p[i]).sum()
}

fn mean_and_variance<const N: usize>(x: &[f64; N], p: &[f64; N]) -> (f64, f64) {
    let mu = mean(x, p);
    let var = (0..N).map(|i| p[i] * (x[i] - mu) * (x[i] - mu)).sum();
    (mu, var)
}

// Solves sum_i p_i * c_i / (1 + theta * c_i) = 0 for theta. The left side
// decreases from +infinity to -infinity between the two poles, so bisection
// finds the one root. Needs some c_i below zero and some above.
fn solve_theta<const N: usize>(p: &[f64; N], c: &[f64; N]) -> Option<f64> {
    let cmin = c.iter().copied().fold(f64::INFINITY, f64::min);
    let cmax = c.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !(cmin < 0.0 && cmax > 0.0) {
        return None;
    }

    let f = |theta: f64| -> f64 { (0..N).map(|i| p[i] * c[i] / (1.0 + theta * c[i])).sum() };

    let epsilon = 1e-9;
    let mut lo = -1.0 / cmax + epsilon;
    let mut hi = -1.0 / cmin - epsilon;
    for _ in 0..300 {
        let mid = 0.5 * (lo + hi);
        if mid == lo || mid == hi {
            break;
        }
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

// The generalised LLR of "expected score is s1" against "... is s0", for the
// observed distribution `probs` over `scores`. Each hypothesis is replaced by
// the maximum likelihood distribution with that expected score.
fn llr_logistic<const N: usize>(
    total: f64,
    scores: &[f64; N],
    probs: &[f64; N],
    s0: f64,
    s1: f64,
) -> Option<f64> {
    let mle = |s: f64| -> Option<[f64; N]> {
        let c: [f64; N] = std::array::from_fn(|i| scores[i] - s);
        let theta = solve_theta(probs, &c)?;
        Some(std::array::from_fn(|i| probs[i] / (1.0 + theta * c[i])))
    };

    let (p0, p1) = (mle(s0)?, mle(s1)?);
    let lpr: [f64; N] = std::array::from_fn(|i| p1[i].ln() - p0[i].ln());
    Some(total * mean(&lpr, probs))
}

// As above, but the hypotheses are about the t-value (mu - 0.5) / sigma
// instead of the expected score. The maximum likelihood distribution is found
// by iteration, starting from the uniform distribution.
fn llr_normalized<const N: usize>(
    total: f64,
    scores: &[f64; N],
    probs: &[f64; N],
    t0: f64,
    t1: f64,
) -> Option<f64> {
    let mle = |t_star: f64| -> Option<[f64; N]> {
        let mu_ref = 0.5;
        let mut p = [1.0 / N as f64; N];

        for _ in 0..10 {
            let (mu, var) = mean_and_variance(scores, &p);
            let sigma = var.sqrt();
            let phi: [f64; N] = std::array::from_fn(|i| {
                let z = (scores[i] - mu) / sigma;
                scores[i] - mu_ref - 0.5 * t_star * sigma * (1.0 + z * z)
            });
            let theta = solve_theta(probs, &phi)?;

            let mut max_diff: f64 = 0.0;
            for i in 0..N {
                let new_p = probs[i] / (1.0 + theta * phi[i]);
                max_diff = max_diff.max((new_p - p[i]).abs());
                p[i] = new_p;
            }
            if max_diff < 1e-4 {
                break;
            }
        }

        Some(p)
    };

    let (p0, p1) = (mle(t0)?, mle(t1)?);
    let lpr: [f64; N] = std::array::from_fn(|i| p1[i].ln() - p0[i].ln());
    Some(total * mean(&lpr, probs))
}

pub mod trinomial {
    use super::*;

    // LLR for win/draw/loss results. All three models are available.
    #[must_use]
    pub fn get_llr(wld: &WLD, elo0: f32, elo1: f32, model: Model) -> f32 {
        if wld.played() == 0 {
            return 0.0;
        }

        let (elo0, elo1) = (f64::from(elo0), f64::from(elo1));
        let (l, d, w) = (regularise(wld.l), regularise(wld.d), regularise(wld.w));
        let total = l + d + w;
        let probs = [l / total, d / total, w / total];
        let scores = [0.0, 0.5, 1.0];

        let llr = match model {
            Model::Normalized => llr_normalized(
                total,
                &scores,
                &probs,
                elo0 / nelo_divided_by_nt(),
                elo1 / nelo_divided_by_nt(),
            ),
            Model::Bayesian => {
                if wld.w == 0 || wld.l == 0 {
                    return 0.0;
                }
                let (pl, pw) = (probs[0], probs[2]);
                let drawelo = 200.0 * ((1.0 - pl) / pl * (1.0 - pw) / pw).log10();
                llr_logistic(
                    total,
                    &scores,
                    &probs,
                    bayeselo_to_score(elo0, drawelo),
                    bayeselo_to_score(elo1, drawelo),
                )
            }
            Model::Logistic => llr_logistic(
                total,
                &scores,
                &probs,
                logistic_elo_to_score(elo0),
                logistic_elo_to_score(elo1),
            ),
        };

        llr.unwrap_or(0.0) as f32
    }
}

pub mod pentanomial {
    use super::*;

    // LLR for pair results. The Bayesian model isn't defined for these; as in
    // fastchess anything other than Normalized is treated as Logistic, but
    // settings validation rejects Bayesian before it gets here.
    #[must_use]
    pub fn get_llr(pairs: &WLDPairs, elo0: f32, elo1: f32, model: Model) -> f32 {
        if pairs.played() == 0 {
            return 0.0;
        }

        let (elo0, elo1) = (f64::from(elo0), f64::from(elo1));
        // The pair scores 0, 1/4, 1/2, 3/4 and 1 are ll, (dl + ld),
        // (wl + lw + dd), (wd + dw) and ww
        let counts = [
            regularise(pairs.ll),
            regularise(pairs.dl + pairs.ld),
            regularise(pairs.wl + pairs.lw + pairs.dd),
            regularise(pairs.wd + pairs.dw),
            regularise(pairs.ww),
        ];
        let total: f64 = counts.iter().sum();
        let probs = counts.map(|c| c / total);
        let scores = [0.0, 0.25, 0.5, 0.75, 1.0];

        let llr = match model {
            Model::Normalized => {
                let sqrt2 = 2.0_f64.sqrt();
                llr_normalized(
                    total,
                    &scores,
                    &probs,
                    sqrt2 * elo0 / nelo_divided_by_nt(),
                    sqrt2 * elo1 / nelo_divided_by_nt(),
                )
            }
            Model::Logistic | Model::Bayesian => llr_logistic(
                total,
                &scores,
                &probs,
                logistic_elo_to_score(elo0),
                logistic_elo_to_score(elo1),
            ),
        };

        llr.unwrap_or(0.0) as f32
    }
}

#[must_use]
pub fn get_lbound(alpha: f32, beta: f32) -> f32 {
    (beta / (1.0 - alpha)).ln()
}

#[must_use]
pub fn get_ubound(alpha: f32, beta: f32) -> f32 {
    ((1.0 - beta) / alpha).ln()
}

#[must_use]
pub fn should_stop(llr: f32, alpha: f32, beta: f32) -> bool {
    llr <= get_lbound(alpha, beta) || llr >= get_ubound(alpha, beta)
}

// How far the llr has travelled towards whichever bound it's heading for,
// as a percentage. 100% means the test would stop.
#[must_use]
pub fn get_progress(llr: f32, lbound: f32, ubound: f32) -> f32 {
    if llr >= 0.0 {
        100.0 * llr / ubound
    } else {
        100.0 * llr / lbound
    }
}

#[cfg(test)]
mod reference;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprt_bounds() {
        assert!((get_lbound(0.05, 0.05) + 2.94).abs() <= 0.01);
        assert!((get_ubound(0.05, 0.05) - 2.94).abs() <= 0.01);
    }

    #[test]
    fn stops_at_the_bounds() {
        assert!(should_stop(-3.0, 0.05, 0.05));
        assert!(should_stop(3.0, 0.05, 0.05));
        assert!(!should_stop(0.0, 0.05, 0.05));
        assert!(!should_stop(2.9, 0.05, 0.05));
    }

    #[test]
    fn progress_towards_the_nearer_bound() {
        assert_eq!(get_progress(0.0, -3.0, 3.0), 0.0);
        assert_eq!(get_progress(1.5, -3.0, 3.0), 50.0);
        assert_eq!(get_progress(-1.5, -3.0, 3.0), 50.0);
        assert_eq!(get_progress(3.0, -3.0, 3.0), 100.0);
    }

    #[test]
    fn model_names_round_trip() {
        for model in [Model::Normalized, Model::Logistic, Model::Bayesian] {
            assert_eq!(Model::from_name(model.name()), Some(model));
        }
        assert_eq!(Model::from_name("nonsense"), None);
        assert_eq!(Model::default(), Model::Normalized);
    }

    #[test]
    fn nothing_played_is_zero() {
        for model in [Model::Normalized, Model::Logistic, Model::Bayesian] {
            assert_eq!(trinomial::get_llr(&WLD::default(), 0.0, 10.0, model), 0.0);
        }
        for model in [Model::Normalized, Model::Logistic] {
            assert_eq!(
                pentanomial::get_llr(&WLDPairs::default(), 0.0, 10.0, model),
                0.0
            );
        }
    }

    #[test]
    fn bayesian_without_both_a_win_and_a_loss_is_zero() {
        let all_lost = WLD { w: 0, l: 20, d: 0 };
        assert_eq!(
            trinomial::get_llr(&all_lost, 0.0, 10.0, Model::Bayesian),
            0.0
        );
    }

    // The sign of the LLR follows which hypothesis the results favour
    #[test]
    fn llr_sign_matches_direction() {
        let winning = WLD {
            w: 60,
            l: 30,
            d: 40,
        };
        let losing = WLD {
            w: 30,
            l: 60,
            d: 40,
        };
        for model in [Model::Normalized, Model::Logistic, Model::Bayesian] {
            assert!(trinomial::get_llr(&winning, 0.0, 10.0, model) > 0.0);
            assert!(trinomial::get_llr(&losing, 0.0, 10.0, model) < 0.0);
        }

        let winning = WLDPairs {
            ww: 20,
            wd: 5,
            ..Default::default()
        };
        let losing = WLDPairs {
            ll: 20,
            dl: 5,
            ..Default::default()
        };
        for model in [Model::Normalized, Model::Logistic] {
            assert!(pentanomial::get_llr(&winning, 0.0, 10.0, model) > 0.0);
            assert!(pentanomial::get_llr(&losing, 0.0, 10.0, model) < 0.0);
        }
    }

    // A single game or pair is nowhere near enough to stop a test
    #[test]
    fn one_result_does_not_stop_a_test() {
        for model in [Model::Normalized, Model::Logistic, Model::Bayesian] {
            for wld in [
                WLD { w: 1, l: 0, d: 0 },
                WLD { w: 0, l: 1, d: 0 },
                WLD { w: 0, l: 0, d: 1 },
            ] {
                let llr = trinomial::get_llr(&wld, 0.0, 10.0, model);
                assert!(!should_stop(llr, 0.05, 0.05), "{model:?} {llr}");
            }
        }
        for model in [Model::Normalized, Model::Logistic] {
            for pairs in [
                WLDPairs {
                    ww: 1,
                    ..Default::default()
                },
                WLDPairs {
                    ll: 1,
                    ..Default::default()
                },
                WLDPairs {
                    dd: 1,
                    ..Default::default()
                },
            ] {
                let llr = pentanomial::get_llr(&pairs, 0.0, 10.0, model);
                assert!(!should_stop(llr, 0.05, 0.05), "{model:?} {llr}");
            }
        }
    }
}
