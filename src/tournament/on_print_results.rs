use crate::{
    elo::{pentanomial, trinomial},
    sprt,
    tournament::{PlayerStatistics, TournamentSettings, TournamentStatistics},
};
use std::collections::HashMap;

pub fn on_print_results(
    tournament_stats: &TournamentStatistics,
    player_stats: &HashMap<usize, PlayerStatistics>,
    settings: &TournamentSettings,
) {
    debug_assert!(player_stats.len() >= 2);
    debug_assert!(tournament_stats.games_completed > 0);

    let _is_duel = player_stats.len() == 2;
    let print_short = tournament_stats.games_completed < 10;

    if print_short {
        let wld = &player_stats.get(&0).unwrap().wld;
        println!(
            "Score of {} vs {}: {} - {} - {} [{:.3}] {}",
            player_stats.get(&0).unwrap().name,
            player_stats.get(&1).unwrap().name,
            wld.w,
            wld.l,
            wld.d,
            wld.winrate(),
            tournament_stats.games_completed
        );
    } else {
        if tournament_stats.games_completed != 10 && !settings.verbose {
            println!();
        }

        let wld = &player_stats.get(&0).unwrap().wld;
        println!(
            "Score of {} vs {}: {} - {} - {} [{:.3}] {}",
            player_stats.get(&0).unwrap().name,
            player_stats.get(&1).unwrap().name,
            wld.w,
            wld.l,
            wld.d,
            wld.winrate(),
            tournament_stats.games_completed
        );
        // if settings.sprt_trinomial {
        let (elo, err) = trinomial::elo_err(&wld);
        println!("Elo difference:  {:.1} +/- {:.1}", elo, err);
        // }
        // if settings.sprt_pentanomial {
        //     let (elo, err) = pentanomial::elo_err(0, 0, 0, 0, 0, 0, 0, 0, 0);
        //     println!("nElo difference:  {:.1} +/- {:.1}", elo, err);
        // }
        if let Some(tri) = &settings.sprt_trinomial {
            println!(
                "SPRT: llr {:.3} ({:.1}%), lbound {:.2}, ubound {:.2}",
                sprt::get_llr(&wld, tri.elo0, tri.elo1),
                0.0,
                sprt::get_lbound(tri.alpha, tri.beta),
                sprt::get_ubound(tri.alpha, tri.beta),
            );
        }
        if let Some(penta) = &settings.sprt_pentanomial {
            println!(
                "nSPRT: llr {:.3} ({:.1}%), lbound {:.2}, ubound {:.2}",
                sprt::get_llr(&wld, penta.elo0, penta.elo1),
                0.0,
                sprt::get_lbound(penta.alpha, penta.beta),
                sprt::get_ubound(penta.alpha, penta.beta),
            );
        }
    }
}
