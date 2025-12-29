mod trinomial {
    #[must_use]
    pub fn elo(_w: i32, _l: i32, _d: i32) -> Option<f32> {
        None
    }
}

mod pentanomial {
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
    ) -> Option<f32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid() {
        assert_eq!(trinomial::elo(0, 0, 0), None);
    }
}
