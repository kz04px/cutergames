use crate::tournament::{
    Event, GameResult, PlayerStatistics, TournamentSettings, TournamentStatistics,
};
use std::{collections::HashMap, sync::mpsc::Sender};

pub fn on_game_finish(
    send: Sender<Event>,
    data: &GameResult,
    player_stats: &mut HashMap<usize, PlayerStatistics>,
    tournament_stats: &mut TournamentStatistics,
    settings: &TournamentSettings,
) {
    debug_assert_ne!(data.player1, data.player2);

    // Update player statistics
    match data.outcome.as_str() {
        "p1win" => {
            player_stats.get_mut(&data.player1).expect("asd").played += 1;
            player_stats.get_mut(&data.player2).expect("asd").played += 1;
            player_stats.get_mut(&data.player1).expect("asd").wins += 1;
            player_stats.get_mut(&data.player2).expect("asd").losses += 1;
        }
        "p2win" => {
            player_stats.get_mut(&data.player1).expect("asd").played += 1;
            player_stats.get_mut(&data.player2).expect("asd").played += 1;
            player_stats.get_mut(&data.player2).expect("asd").wins += 1;
            player_stats.get_mut(&data.player1).expect("asd").losses += 1;
        }
        "draw" => {
            player_stats.get_mut(&data.player1).expect("asd").played += 1;
            player_stats.get_mut(&data.player2).expect("asd").played += 1;
            player_stats.get_mut(&data.player1).expect("asd").draws += 1;
            player_stats.get_mut(&data.player2).expect("asd").draws += 1;
        }
        msg => {
            println!("Error: {}", msg);
            return;
        }
    }

    tournament_stats.games_completed += 1;

    let mut should_stop = false;
    let is_sprt_pass = false;
    let is_sprt_fail = false;
    let is_sprt_stop = is_sprt_pass | is_sprt_fail;

    // Should stop - sprt
    if settings.sprt_trinomial || settings.sprt_pentanomial {
        should_stop |= is_sprt_stop;
    }

    // Should stop - number of games
    if let Some(num) = settings.num_games {
        should_stop |= tournament_stats.games_completed >= num;
    }

    let should_print = tournament_stats.games_completed <= 10
        || tournament_stats.games_completed % 10 == 0
        || should_stop;

    if settings.verbose {
        println!(
            "Finish game #{} -- {} vs {} -- result: {}",
            data.id,
            player_stats.get(&data.player1).unwrap().name,
            player_stats.get(&data.player2).unwrap().name,
            data.outcome
        );
    }

    if should_print {
        let w = player_stats.get(&0).unwrap().wins;
        let l = player_stats.get(&0).unwrap().losses;
        let d = player_stats.get(&0).unwrap().draws;
        let ratio = (2 * w + d) as f32 / (2 * tournament_stats.games_completed) as f32;
        println!("");
        println!(
            "Score of {} vs {}: {} - {} - {} [{:.3}] {}",
            player_stats.get(&0).unwrap().name,
            player_stats.get(&1).unwrap().name,
            w,
            l,
            d,
            ratio,
            tournament_stats.games_completed
        );
        println!("Elo difference: {} +/- {}", 0.0, 0.0);
        println!(
            "SPRT: llr {} ({}%), lbound {}, ubound {}",
            0.0, 0.0, 0.0, 0.0
        );
    }

    if should_stop {
        let _ = send.send(Event::TournamentFinish);
    }
}
