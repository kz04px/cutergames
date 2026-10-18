use crate::stats::WLDPairs;
use crate::tournament::{
    PlayerStatistics, TournamentSettings, TournamentStatistics, play::GameOutcome,
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Res {
    W,
    L,
    D,
}

impl Res {
    fn flip(self) -> Self {
        match self {
            Self::W => Self::L,
            Self::L => Self::W,
            Self::D => Self::D,
        }
    }
}

fn bump(pairs: &mut WLDPairs, first: Res, second: Res) {
    use Res::*;
    match (first, second) {
        (W, W) => pairs.ww += 1,
        (W, L) => pairs.wl += 1,
        (W, D) => pairs.wd += 1,
        (D, W) => pairs.dw += 1,
        (D, L) => pairs.dl += 1,
        (D, D) => pairs.dd += 1,
        (L, W) => pairs.lw += 1,
        (L, L) => pairs.ll += 1,
        (L, D) => pairs.ld += 1,
    }
}

// `p1` played the first game as player 1 and the second as player 2
pub fn on_game_pair_result(
    settings: &TournamentSettings,
    player_stats: &mut HashMap<usize, PlayerStatistics>,
    tournament_stats: &mut TournamentStatistics,
    p1: usize,
    p2: usize,
    r1: Option<GameOutcome>,
    r2: Option<GameOutcome>,
) {
    debug_assert_ne!(p1, p2);

    if settings.verbose {
        println!("<Event::GamePairResult> Pair complete");
    }

    let (Some(r1), Some(r2)) = (r1, r2) else {
        return;
    };

    // Both games from p1's point of view
    let first = match r1 {
        GameOutcome::P1win => Res::W,
        GameOutcome::P2win => Res::L,
        GameOutcome::Draw => Res::D,
    };
    let second = match r2 {
        GameOutcome::P1win => Res::L,
        GameOutcome::P2win => Res::W,
        GameOutcome::Draw => Res::D,
    };

    bump(&mut player_stats.get_mut(&p1).unwrap().wld_pairs, first, second);
    bump(
        &mut player_stats.get_mut(&p2).unwrap().wld_pairs,
        first.flip(),
        second.flip(),
    );

    let matchup = tournament_stats
        .matchups
        .entry((p1.min(p2), p1.max(p2)))
        .or_default();
    if p1 < p2 {
        bump(&mut matchup.wld_pairs, first, second);
    } else {
        bump(&mut matchup.wld_pairs, first.flip(), second.flip());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_updates_players_and_matchup() {
        let settings = TournamentSettings::default();
        let mut players: HashMap<usize, PlayerStatistics> = (0..3)
            .map(|i| (i, PlayerStatistics::default()))
            .collect();
        let mut tournament = TournamentStatistics::default();

        // Player 2 beats player 1 in both colours
        on_game_pair_result(
            &settings,
            &mut players,
            &mut tournament,
            2,
            1,
            Some(GameOutcome::P1win),
            Some(GameOutcome::P2win),
        );

        assert_eq!(players[&2].wld_pairs.ww, 1);
        assert_eq!(players[&1].wld_pairs.ll, 1);
        // From player 1's point of view, as the lower index
        assert_eq!(tournament.matchups[&(1, 2)].wld_pairs.ll, 1);
        assert_eq!(tournament.matchups[&(1, 2)].wld_pairs.ww, 0);
    }
}
