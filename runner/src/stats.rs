use serde::{Deserialize, Serialize};

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct WLD {
    pub w: usize,
    pub l: usize,
    pub d: usize,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct WLDPairs {
    pub ww: usize,
    pub wl: usize,
    pub wd: usize,
    pub dw: usize,
    pub dl: usize,
    pub dd: usize,
    pub lw: usize,
    pub ll: usize,
    pub ld: usize,
}

impl WLD {
    #[must_use]
    pub fn new() -> Self {
        Self { w: 0, l: 0, d: 0 }
    }

    #[must_use]
    pub fn played(&self) -> usize {
        self.w + self.l + self.d
    }

    #[must_use]
    pub fn winrate(&self) -> f32 {
        (2 * self.w + self.d) as f32 / (2.0 * self.played() as f32)
    }
}

impl WLDPairs {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ww: 0,
            wl: 0,
            wd: 0,
            dw: 0,
            dl: 0,
            dd: 0,
            lw: 0,
            ll: 0,
            ld: 0,
        }
    }

    #[must_use]
    pub fn played(&self) -> usize {
        2 * (self.ww
            + self.wl
            + self.wd
            + self.lw
            + self.ll
            + self.ld
            + self.dw
            + self.dl
            + self.dd)
    }

    #[must_use]
    pub fn points(&self) -> f32 {
        2.0 * self.ww as f32
            + 1.5 * (self.wd + self.dw) as f32
            + 1.0 * (self.wl + self.lw + self.dd) as f32
            + 0.5 * (self.dl + self.ld) as f32
    }

    #[must_use]
    pub fn winrate(&self) -> f32 {
        self.points() / self.played() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wld_winrate() {
        assert_eq!(WLD { w: 1, l: 0, d: 0 }.winrate(), 1.0);
        assert_eq!(WLD { w: 0, l: 1, d: 0 }.winrate(), 0.0);
        assert_eq!(WLD { w: 0, l: 0, d: 1 }.winrate(), 0.5);
        assert_eq!(WLD { w: 1, l: 1, d: 0 }.winrate(), 0.5);
        assert_eq!(WLD { w: 3, l: 1, d: 0 }.winrate(), 0.75);
    }

    #[test]
    fn wld_pairs_winrate() {
        let won_both = WLDPairs {
            ww: 1,
            ..Default::default()
        };
        assert_eq!(won_both.played(), 2);
        assert_eq!(won_both.points(), 2.0);
        assert_eq!(won_both.winrate(), 1.0);

        assert_eq!(
            WLDPairs {
                ll: 1,
                ..Default::default()
            }
            .winrate(),
            0.0
        );
        assert_eq!(
            WLDPairs {
                dd: 1,
                ..Default::default()
            }
            .winrate(),
            0.5
        );
        assert_eq!(
            WLDPairs {
                wl: 1,
                ..Default::default()
            }
            .winrate(),
            0.5
        );
        assert_eq!(
            WLDPairs {
                wd: 1,
                ..Default::default()
            }
            .winrate(),
            0.75
        );
        assert_eq!(
            WLDPairs {
                ww: 1,
                ll: 1,
                ..Default::default()
            }
            .winrate(),
            0.5
        );
    }
}
