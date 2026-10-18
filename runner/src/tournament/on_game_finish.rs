use crate::tournament::on_game_pair_result::on_game_pair_result;
use crate::{
    sprt,
    tournament::{
        AbortKind, Event, GameData, PlayerStatistics, TournamentSettings, TournamentStatistics,
        play::GameOutcome,
    },
};
use std::{collections::HashMap, sync::mpsc::Sender};

pub fn on_game_finish(
    send: Sender<Event>,
    data: &GameData,
    pair_store: &mut HashMap<usize, GameOutcome>,
    player_stats: &mut HashMap<usize, PlayerStatistics>,
    tournament_stats: &mut TournamentStatistics,
    settings: &TournamentSettings,
) {
    // Can't play against ourselves
    debug_assert_ne!(data.player1, data.player2);

    // Current game ID can't already exist
    debug_assert!(!pair_store.contains_key(&data.id));

    if settings.verbose {
        println!(
            "<Event::GameFinish> Finish game #{} -- {} vs {} -- result: {:?}",
            data.id,
            player_stats.get(&data.player1).unwrap().name,
            player_stats.get(&data.player2).unwrap().name,
            data.outcome
        );
    }

    // Update player statistics
    player_stats
        .get_mut(&data.player1)
        .expect("player_stats has an entry for every player")
        .played += 1;
    player_stats
        .get_mut(&data.player2)
        .expect("player_stats has an entry for every player")
        .played += 1;
    match data.outcome {
        Some(GameOutcome::P1win) => {
            player_stats
                .get_mut(&data.player1)
                .expect("player_stats has an entry for every player")
                .wld
                .w += 1;
            player_stats
                .get_mut(&data.player2)
                .expect("player_stats has an entry for every player")
                .wld
                .l += 1;
        }
        Some(GameOutcome::P2win) => {
            player_stats
                .get_mut(&data.player1)
                .expect("player_stats has an entry for every player")
                .wld
                .l += 1;
            player_stats
                .get_mut(&data.player2)
                .expect("player_stats has an entry for every player")
                .wld
                .w += 1;
        }
        Some(GameOutcome::Draw) => {
            player_stats
                .get_mut(&data.player1)
                .expect("player_stats has an entry for every player")
                .wld
                .d += 1;
            player_stats
                .get_mut(&data.player2)
                .expect("player_stats has an entry for every player")
                .wld
                .d += 1;
        }
        None => {
            // The game counts as unplayed, so the partner's result stays in
            // the store for when a resumed match plays it again
            println!("<Error> Game {} finished without result", data.id);
            return;
        }
    }

    // Attribute the abort, if any, to the responsible player
    if let Some((player, kind)) = data.abort {
        let stats = player_stats
            .get_mut(&player)
            .expect("player_stats has an entry for every player");
        match kind {
            AbortKind::Timeout => stats.timeouts += 1,
            AbortKind::IllegalMove => stats.illegalmoves += 1,
            AbortKind::Crash => stats.crashes += 1,
        }
    }

    // Update tournament statistics
    tournament_stats.games_completed += 1;
    tournament_stats.total_ply += data.ply;

    let mut should_stop = false;
    let is_duel = settings.players.len() == 2;

    // SPRT
    let is_trinomial_stop = if is_duel && let Some(tri) = &settings.sprt_trinomial {
        tri.autostop
            && sprt::should_stop(
                sprt::trinomial::get_llr(
                    &player_stats.get(&0).expect("Missing player 0").wld,
                    tri.elo0,
                    tri.elo1,
                    tri.model,
                ),
                tri.alpha,
                tri.beta,
            )
    } else {
        false
    };
    let is_pentanomial_stop = if is_duel && let Some(penta) = &settings.sprt_pentanomial {
        penta.autostop
            && sprt::should_stop(
                sprt::pentanomial::get_llr(
                    &player_stats.get(&0).expect("Missing player 0").wld_pairs,
                    penta.elo0,
                    penta.elo1,
                    penta.model,
                ),
                penta.alpha,
                penta.beta,
            )
    } else {
        false
    };

    // Should stop - sprt
    should_stop |= is_trinomial_stop || is_pentanomial_stop;

    // Should stop - number of games
    if let Some(num) = settings.num_games {
        should_stop |= tournament_stats.games_completed >= num;
    }

    // Did we complete a game pair?
    let is_first_game = data.id.is_multiple_of(2);
    let partner_id = if is_first_game {
        data.id + 1
    } else {
        data.id - 1
    };
    let is_complete_pair = pair_store.contains_key(&partner_id);

    // Update game pairs
    if is_complete_pair {
        let first_player = if is_first_game {
            data.player1
        } else {
            data.player2
        };
        let second_player = if is_first_game {
            data.player2
        } else {
            data.player1
        };
        let first_result = if is_first_game {
            data.outcome
        } else {
            pair_store.remove(&partner_id)
        };
        let second_result = if is_first_game {
            pair_store.remove(&partner_id)
        } else {
            data.outcome
        };

        // Recorded here rather than through an event, so the pair is never
        // missing from both the store and the pair statistics when the
        // results file is written
        on_game_pair_result(
            settings,
            player_stats,
            first_player,
            second_player,
            first_result,
            second_result,
        );
    } else {
        pair_store.insert(data.id, data.outcome.unwrap());
    }

    // Print results update?
    let should_print = tournament_stats.games_completed <= settings.update_frequency
        || tournament_stats
            .games_completed
            .is_multiple_of(settings.update_frequency)
        || should_stop;
    if should_print {
        let _ = send.send(Event::PrintUpdate);
    }

    if should_stop {
        let _ = send.send(Event::TournamentFinish);
    }
}
