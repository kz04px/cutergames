use crate::tournament::{PlayerStatistics, TournamentSettings, play::GameOutcome};
use std::collections::HashMap;

pub fn on_game_pair_result(
    settings: &TournamentSettings,
    player_stats: &mut HashMap<usize, PlayerStatistics>,
    p1: usize,
    p2: usize,
    r1: Option<GameOutcome>,
    r2: Option<GameOutcome>,
) {
    debug_assert_ne!(p1, p2);

    if settings.verbose {
        println!("<Event::GamePairResult> Pair complete");
    }

    match (r1, r2) {
        (Some(GameOutcome::P1win), Some(GameOutcome::P1win)) => {
            player_stats.get_mut(&p1).unwrap().wld_pairs.wl += 1;
            player_stats.get_mut(&p2).unwrap().wld_pairs.lw += 1;
        }
        (Some(GameOutcome::P1win), Some(GameOutcome::P2win)) => {
            player_stats.get_mut(&p1).unwrap().wld_pairs.ww += 1;
            player_stats.get_mut(&p2).unwrap().wld_pairs.ll += 1;
        }
        (Some(GameOutcome::P1win), Some(GameOutcome::Draw)) => todo!("Uh oh 1"),

        (Some(GameOutcome::P2win), Some(GameOutcome::P1win)) => {
            player_stats.get_mut(&p1).unwrap().wld_pairs.ll += 1;
            player_stats.get_mut(&p2).unwrap().wld_pairs.ww += 1;
        }
        (Some(GameOutcome::P2win), Some(GameOutcome::P2win)) => {
            player_stats.get_mut(&p1).unwrap().wld_pairs.lw += 1;
            player_stats.get_mut(&p2).unwrap().wld_pairs.wl += 1;
        }
        (Some(GameOutcome::P2win), Some(GameOutcome::Draw)) => todo!("Uh oh 2"),

        (Some(GameOutcome::Draw), Some(GameOutcome::P1win)) => todo!("Uh oh 3"),
        (Some(GameOutcome::Draw), Some(GameOutcome::P2win)) => todo!("Uh oh 4"),
        (Some(GameOutcome::Draw), Some(GameOutcome::Draw)) => todo!("Uh oh 5"),

        (_, _) => {}
    }
}
