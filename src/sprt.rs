use crate::stats::WLD;

#[must_use]
fn elo_to_probability(elo: f32, drawelo: f32) -> (f32, f32, f32) {
    let pwin = 1.0 / (1.0 + 10.0_f32.powf((drawelo - elo) / 400.0));
    let ploss = 1.0 / (1.0 + 10.0_f32.powf((drawelo + elo) / 400.0));
    let pdraw = 1.0 - pwin - ploss;
    debug_assert!(0.0 <= pwin && pwin <= 1.0, "pwin: {}", pwin);
    debug_assert!(0.0 <= ploss && ploss <= 1.0, "ploss: {}", ploss);
    debug_assert!(0.0 <= pdraw && pdraw <= 1.0, "pdraw: {}", pdraw);
    (pwin, pdraw, ploss)
}

#[must_use]
fn probability_to_elo(pwin: f32, _: f32, ploss: f32) -> (f32, f32) {
    debug_assert!(0.0 <= pwin && pwin <= 1.0, "pwin: {}", pwin);
    debug_assert!(0.0 <= ploss && ploss <= 1.0, "ploss: {}", ploss);
    let elo = 200.0 * (pwin / ploss * (1.0 - ploss) / (1.0 - pwin)).log10();
    let draw_elo = 200.0 * ((1.0 - ploss) / ploss * (1.0 - pwin) / pwin).log10();
    (elo, draw_elo)
}

#[must_use]
pub fn get_llr(wld: &WLD, elo0: f32, elo1: f32) -> f32 {
    let wins = wld.w as f32 + 0.5;
    let losses = wld.l as f32 + 0.5;
    let draws = wld.d as f32 + 0.5;

    let total = wins + losses + draws;

    let (_, drawelo) = probability_to_elo(wins / total, draws / total, losses / total);

    let x = 10.0f32.powf(-drawelo / 400.0);
    let s = 4.0 * x / ((1.0 + x) * (1.0 + x));

    let (p0win, p0draw, p0loss) = elo_to_probability(elo0 / s, drawelo);
    let (p1win, p1draw, p1loss) = elo_to_probability(elo1 / s, drawelo);

    let wins_factor = wins * (p1win / p0win).ln();
    let losses_factor = losses * (p1loss / p0loss).ln();
    let draws_factor = draws * (p1draw / p0draw).ln();

    wins_factor + losses_factor + draws_factor
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
pub fn should_stop(wld: &WLD, elo0: f32, elo1: f32, alpha: f32, beta: f32) -> bool {
    let llr = get_llr(wld, elo0, elo1);
    let lbound = get_lbound(alpha, beta);
    let ubound = get_ubound(alpha, beta);
    return llr <= lbound || llr >= ubound;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprt_bounds() {
        debug_assert!((get_lbound(0.05, 0.05) + 2.94).abs() <= 0.01);
        debug_assert!((get_ubound(0.05, 0.05) - 2.94).abs() <= 0.01);
    }

    #[test]
    fn sprt_llr() {
        let tests = [
            ((7, 3, 0, 0.0, 10.0), 0.116),
            ((12, 6, 2, 0.0, 10.0), 0.188),
            ((20, 8, 2, 0.0, 10.0), 0.367),
            ((25, 11, 4, 0.0, 10.0), 0.440),
            ((29, 14, 7, 0.0, 10.0), 0.489),
            ((36, 15, 9, 0.0, 10.0), 0.703),
            ((41, 17, 12, 0.0, 10.0), 0.825),
            ((47, 18, 15, 0.0, 10.0), 1.027),
            ((53, 21, 16, 0.0, 10.0), 1.114),
            ((55, 26, 19, 0.0, 10.0), 1.004),
            ((127, 47, 46, 0.0, 10.0), 2.925),
            ((133, 48, 49, 0.0, 10.0), 3.135),
            ((191, 61, 58, 0.0, 10.0), 4.695),
        ];

        for ((w, l, d, elo0, elo1), expected) in tests {
            let got = get_llr(&WLD { w, l, d }, elo0, elo1);
            let diff = (got - expected).abs();
            debug_assert!(diff <= 0.001);
        }
    }

    #[test]
    fn sprt_stop() {
        let tests = [
            ((0, 0, 0, 0.0, 10.0), false),
            ((7, 3, 0, 0.0, 10.0), false),
            ((12, 6, 2, 0.0, 10.0), false),
            ((20, 8, 2, 0.0, 10.0), false),
            ((25, 11, 4, 0.0, 10.0), false),
            ((29, 14, 7, 0.0, 10.0), false),
            ((36, 15, 9, 0.0, 10.0), false),
            ((41, 17, 12, 0.0, 10.0), false),
            ((47, 18, 15, 0.0, 10.0), false),
            ((53, 21, 16, 0.0, 10.0), false),
            ((55, 26, 19, 0.0, 10.0), false),
            ((127, 47, 46, 0.0, 10.0), false),
            ((133, 48, 49, 0.0, 10.0), true),
            ((191, 61, 58, 0.0, 10.0), true),
        ];

        for ((w, l, d, elo0, elo1), expected) in tests {
            let stopped = should_stop(&WLD { w, l, d }, elo0, elo1, 0.05, 0.05);
            debug_assert_eq!(expected, stopped);
        }
    }
}
