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
        {
            let (elo, err) = trinomial::elo_err(wld);
            let (nelo, nerr) = trinomial::nelo_err(wld);
            println!(
                "Trinomial:   Elo: {:.1} +/- {:.1}, nElo: {:.1} +/- {:.1}",
                elo, err, nelo, nerr
            );
        }
        {
            let wld_pairs = &player_stats.get(&0).unwrap().wld_pairs;
            let (elo, err) = pentanomial::elo_err(wld_pairs);
            let (nelo, nerr) = pentanomial::nelo_err(wld_pairs);
            println!(
                "Pentanomial: Elo: {:.1} +/- {:.1}, nElo: {:.1} +/- {:.1}",
                elo, err, nelo, nerr
            );
        }
        if let Some(tri) = &settings.sprt_trinomial {
            let llr = sprt::trinomial::get_llr(wld, tri.elo0, tri.elo1, tri.model);
            let lbound = sprt::get_lbound(tri.alpha, tri.beta);
            let ubound = sprt::get_ubound(tri.alpha, tri.beta);
            println!(
                "SPRT (trinomial, {}): llr {:.3} ({:.1}%), lbound {:.2}, ubound {:.2}, elo [{}, {}]",
                tri.model.name(),
                llr,
                sprt::get_progress(llr, lbound, ubound),
                lbound,
                ubound,
                tri.elo0,
                tri.elo1,
            );
        }
        if let Some(penta) = &settings.sprt_pentanomial {
            let wld_pairs = &player_stats.get(&0).unwrap().wld_pairs;
            let llr = sprt::pentanomial::get_llr(wld_pairs, penta.elo0, penta.elo1, penta.model);
            let lbound = sprt::get_lbound(penta.alpha, penta.beta);
            let ubound = sprt::get_ubound(penta.alpha, penta.beta);
            println!(
                "SPRT (pentanomial, {}): llr {:.3} ({:.1}%), lbound {:.2}, ubound {:.2}, elo [{}, {}]",
                penta.model.name(),
                llr,
                sprt::get_progress(llr, lbound, ubound),
                lbound,
                ubound,
                penta.elo0,
                penta.elo1,
            );
        }
    }
}
