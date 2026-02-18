#[derive(Default)]
pub struct WLD {
    pub w: usize,
    pub l: usize,
    pub d: usize,
}

#[derive(Default)]
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
    pub fn winrate(&self) -> f32 {
        (4 * self.ww
            + 3 * (self.wd + self.dw)
            + 2 * (self.wl + self.lw + self.dd)
            + (self.dl + self.ld)) as f32
            / (4.0 * self.played() as f32)
    }
}
