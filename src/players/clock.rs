#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockType {
    Time(u128, u128),
    Movetime(u128),
    Depth(u8),
}

impl ClockType {
    // The player key and value that give this clock, e.g. ("tc", "1000+10")
    #[must_use]
    pub fn key_value(self) -> (&'static str, String) {
        match self {
            Self::Time(ms, 0) => ("tc", ms.to_string()),
            Self::Time(ms, inc) => ("tc", format!("{}+{}", ms, inc)),
            Self::Movetime(ms) => ("st", ms.to_string()),
            Self::Depth(depth) => ("depth", depth.to_string()),
        }
    }
}

impl Default for ClockType {
    fn default() -> Self {
        Self::Movetime(10)
    }
}

// Builds the `go` command for the player about to move. `own_clock` is that
// player's clock, which is what movetime and depth limits come from; the
// UGI/UAI time controls always describe both sides, regardless of whose
// turn it is, so those use `p1_clock` and `p2_clock`.
#[must_use]
pub fn go_command(own_clock: ClockType, p1_clock: ClockType, p2_clock: ClockType) -> String {
    match own_clock {
        ClockType::Movetime(ms) => format!("go movetime {}\n", ms),
        ClockType::Depth(depth) => format!("go depth {}\n", depth),
        ClockType::Time(..) => {
            let (ClockType::Time(p1_time, p1_inc), ClockType::Time(p2_time, p2_inc)) =
                (p1_clock, p2_clock)
            else {
                unreachable!("Both players must be using the same clock type");
            };
            format!(
                "go p1time {} p2time {} p1inc {} p2inc {}\n",
                p1_time, p2_time, p1_inc, p2_inc
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movetime() {
        assert_eq!(
            go_command(
                ClockType::Movetime(10),
                ClockType::Movetime(10),
                ClockType::Movetime(10)
            ),
            "go movetime 10\n"
        );
    }

    #[test]
    fn depth() {
        assert_eq!(
            go_command(
                ClockType::Depth(5),
                ClockType::Depth(5),
                ClockType::Depth(5)
            ),
            "go depth 5\n"
        );
    }

    #[test]
    fn time() {
        assert_eq!(
            go_command(
                ClockType::Time(1000, 10),
                ClockType::Time(1000, 10),
                ClockType::Time(2000, 20)
            ),
            "go p1time 1000 p2time 2000 p1inc 10 p2inc 20\n"
        );
    }

    // The player moving second must get its own movetime/depth, not the
    // first player's.
    #[test]
    fn limits_come_from_the_mover_not_player_one() {
        let (a, b) = (ClockType::Movetime(20), ClockType::Movetime(60));
        assert_eq!(go_command(b, a, b), "go movetime 60\n");
        assert_eq!(go_command(a, a, b), "go movetime 20\n");

        let (a, b) = (ClockType::Depth(3), ClockType::Depth(7));
        assert_eq!(go_command(b, a, b), "go depth 7\n");
    }
}
