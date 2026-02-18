pub mod trinomial {
    use crate::stats::WLD;
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

    #[must_use]
    pub fn elo_err(wld: &WLD) -> (f32, f32) {
        (elo(wld), err(wld))
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
            let dev_w = (wld.w as f32 / wld.played() as f32) * (1.0 - m_mu).powf(2.0);
            let dev_l = (wld.l as f32 / wld.played() as f32) * (0.0 - m_mu).powf(2.0);
            let dev_d = (wld.d as f32 / wld.played() as f32) * (0.5 - m_mu).powf(2.0);
            let m_stdev = (dev_w + dev_l + dev_d).sqrt() / (wld.played() as f32).sqrt();
            let mu_min = m_mu + phi_inv(0.025) * m_stdev;
            let mu_max = m_mu + phi_inv(0.975) * m_stdev;
            (diff(mu_max) - diff(mu_min)) / 2.0
        }
    }
}

pub mod pentanomial {
    #[must_use]
    pub fn elo_err(
        ww: i32,
        wl: i32,
        wd: i32,
        lw: i32,
        ll: i32,
        ld: i32,
        dw: i32,
        dl: i32,
        dd: i32,
    ) -> (f32, f32) {
        (
            elo(ww, wl, wd, lw, ll, ld, dw, dl, dd),
            err(ww, wl, wd, lw, ll, ld, dw, dl, dd),
        )
    }

    #[must_use]
    pub fn elo(
        _ww: i32,
        _wl: i32,
        _wd: i32,
        _lw: i32,
        _ll: i32,
        _ld: i32,
        _dw: i32,
        _dl: i32,
        _dd: i32,
    ) -> f32 {
        0.0
    }

    #[must_use]
    pub fn err(
        _ww: i32,
        _wl: i32,
        _wd: i32,
        _lw: i32,
        _ll: i32,
        _ld: i32,
        _dw: i32,
        _dl: i32,
        _dd: i32,
    ) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::WLD;

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
}
