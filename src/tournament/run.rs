use crate::stats::{WLD, WLDPairs};
use crate::tournament::generator::Generator;
use crate::tournament::on_game_finish::on_game_finish;
use crate::tournament::on_game_start::on_game_start;
use crate::tournament::on_player_create::on_player_create;
use crate::tournament::on_player_destroy::on_player_destroy;
use crate::tournament::on_print_results::on_print_results;
use crate::tournament::on_thread_finish::on_thread_finish;
use crate::tournament::on_thread_start::on_thread_start;
use crate::tournament::play::GameOutcome;
use crate::tournament::results_file::{ResultsFile, ResumeState, Status, is_complete};
use crate::tournament::worker::worker;
use crate::tournament::{Event, PlayerStatistics, TournamentSettings, TournamentStatistics};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, sync::mpsc::channel, thread};

pub fn run(settings: &TournamentSettings) -> Result<(), String> {
    let (send, recv) = channel();
    let mut threads = vec![];
    let mut player_stats: HashMap<usize, PlayerStatistics> = HashMap::new();
    let should_stop = Arc::new(AtomicBool::new(false));
    let mut pair_store: HashMap<usize, GameOutcome> = HashMap::new();
    let mut threads_running = 0;
    let mut games_at_last_print = 0;

    // Ctrl-C: the first press lets in-progress games finish, engines shut
    // down normally, and final results print, rather than killing
    // everything outright. Engine subprocesses run in their own process
    // group (see EngineProcess::new) so the terminal's SIGINT doesn't reach
    // them directly -- only this handler decides when to stop. A second
    // press abandons those in-progress games immediately by sending every
    // engine "stop"/"quit"; the workers then see the closed connection,
    // abort their games, and shut down through the same path as normal.
    {
        let nsend = send.clone();
        let interrupted = Arc::new(AtomicBool::new(false));
        ctrlc::set_handler(move || {
            if !interrupted.swap(true, Ordering::Relaxed) {
                println!("\n<Ctrl-C> Finishing in-progress games, then stopping. Press again to end them immediately.");
                let _ = nsend.send(Event::TournamentFinish);
            } else {
                println!("\n<Ctrl-C> Ending in-progress games immediately.");
                crate::players::quit_all_engines();
            }
        })
        .expect("Failed to set Ctrl-C handler");
    }

    // Initialise player statistics
    for idx in 0..settings.players.len() {
        player_stats.insert(
            idx,
            PlayerStatistics {
                name: settings.players.get(idx).unwrap().name.clone(),
                played: 0,
                crashes: 0,
                timeouts: 0,
                illegalmoves: 0,
                wld: WLD::new(),
                wld_pairs: WLDPairs::new(),
            },
        );
    }

    // Statistics
    let mut tournament_stats = TournamentStatistics::default();
    // Indexed by game id, so the games without a result can be saved
    let mut has_result: Vec<bool> = vec![];

    // Work generator
    let mut generator = Generator::new(
        settings.players.len(),
        settings.fens.len(),
        settings.num_games,
    );

    let results_file = match settings.results.as_deref() {
        Some(path) if settings.resume => {
            let (file, state) = ResultsFile::resume(path, settings)?;
            println!(
                "Resuming the match in \"{}\" after {} games",
                path, state.games
            );
            has_result = vec![true; state.next_game];
            for &id in &state.unplayed_games {
                has_result[id] = false;
            }
            generator = generator.resume(state.next_game, state.unplayed_games.clone());
            state.restore(&mut tournament_stats, &mut player_stats, &mut pair_store);
            Some(file)
        }
        Some(path) => Some(ResultsFile::new(path, settings)),
        None => None,
    };
    let generator = Arc::new(Mutex::new(generator));

    let save_results = |tournament_stats: &TournamentStatistics,
                        player_stats: &HashMap<usize, PlayerStatistics>,
                        pair_store: &HashMap<usize, GameOutcome>,
                        has_result: &[bool],
                        finished: bool|
     -> Result<(), String> {
        let Some(file) = &results_file else {
            return Ok(());
        };

        let next_game = generator.lock().expect("Generator lock poisoned").id;
        let unplayed: Vec<usize> = (0..next_game)
            .filter(|&id| !has_result.get(id).copied().unwrap_or(false))
            .collect();
        let status = if !finished {
            Status::Running
        } else if is_complete(settings, player_stats, next_game, &unplayed) {
            Status::Complete
        } else {
            Status::Stopped
        };
        let state = ResumeState::capture(
            tournament_stats,
            player_stats,
            pair_store,
            next_game,
            unplayed,
        );

        file.write(settings, tournament_stats, player_stats, status, &state)
    };
    // A bad path is reported before any games are played rather than after
    save_results(
        &tournament_stats,
        &player_stats,
        &pair_store,
        &has_result,
        false,
    )?;

    // Keypress thread
    let nsend = send.clone();
    thread::spawn(move || {
        loop {
            let mut buffer = String::new();
            match std::io::stdin().read_line(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }

            if nsend.send(Event::KeyPress).is_err() {
                break;
            }
        }
    });

    let _ = send.send(Event::TournamentStart);

    // Worker threads
    for thread_id in 0..settings.num_threads {
        let set = settings.clone();
        let nsend = send.clone();
        let stop = should_stop.clone();
        let generator = generator.clone();
        let thread = thread::spawn(move || worker(generator, thread_id, nsend, &set, stop));
        threads.push(thread);
    }

    let handle_event = |event: Event,
                        pair_store: &mut HashMap<usize, GameOutcome>,
                        player_stats: &mut HashMap<usize, PlayerStatistics>,
                        tournament_stats: &mut TournamentStatistics,
                        threads_running: &mut usize,
                        games_at_last_print: &mut usize,
                        has_result: &mut Vec<bool>|
     -> bool {
        match event {
            // Game
            Event::GameStart(id, fen_idx) => {
                on_game_start(id, &settings.fens[fen_idx], settings.verbose)
            }
            Event::GameFinish(data) => {
                on_game_finish(
                    send.clone(),
                    &data,
                    pair_store,
                    player_stats,
                    tournament_stats,
                    settings,
                );

                if data.outcome.is_some() {
                    if has_result.len() <= data.id {
                        has_result.resize(data.id + 1, false);
                    }
                    has_result[data.id] = true;
                }

                // Saved after every game, so a match that's killed loses as
                // little as possible when it's resumed
                if let Err(err) = save_results(
                    tournament_stats,
                    player_stats,
                    pair_store,
                    has_result,
                    false,
                ) {
                    println!("<Warning> {}", err);
                }
            }
            // Tournament
            Event::TournamentStart => {
                if settings.verbose {
                    println!("<Event::TournamentStart>");
                }
            }
            Event::TournamentFinish => should_stop.store(true, Ordering::Relaxed),
            // Players
            Event::PlayerCreate(id) => on_player_create(id, tournament_stats, settings.verbose),
            Event::PlayerDestroy => on_player_destroy(tournament_stats, settings.verbose),
            // Threads
            Event::ThreadStart(id) => {
                *threads_running += 1;
                on_thread_start(id, settings.verbose);
            }
            Event::ThreadFinish(id) => {
                *threads_running -= 1;
                on_thread_finish(id, settings.verbose);

                // Every worker has finished
                return *threads_running == 0;
            }
            // Results
            Event::PrintUpdate => {
                on_print_results(tournament_stats, player_stats, settings);
                *games_at_last_print = tournament_stats.games_completed;
            }
            // Other -- pressing enter prints the current standings on demand
            Event::KeyPress => {
                if tournament_stats.games_completed > 0 {
                    on_print_results(tournament_stats, player_stats, settings);
                    *games_at_last_print = tournament_stats.games_completed;
                }
            }
        }

        false
    };

    while let Ok(event) = recv.recv() {
        let all_finished = handle_event(
            event,
            &mut pair_store,
            &mut player_stats,
            &mut tournament_stats,
            &mut threads_running,
            &mut games_at_last_print,
            &mut has_result,
        );

        if all_finished {
            break;
        }
    }

    debug_assert_eq!(threads_running, 0);

    // The final results of a game or pair can be queued behind the last
    // Event::ThreadFinish, so handle whatever is left rather than losing it.
    while let Ok(event) = recv.try_recv() {
        handle_event(
            event,
            &mut pair_store,
            &mut player_stats,
            &mut tournament_stats,
            &mut threads_running,
            &mut games_at_last_print,
            &mut has_result,
        );
    }

    for thread in threads {
        let _ = thread.join();
    }

    // Whatever stopped the tournament (Ctrl-C, an SPRT bound, --games) may
    // not have landed on an --updates boundary, so make sure the last
    // update reflects the final state rather than a stale one.
    if tournament_stats.games_completed > 0
        && tournament_stats.games_completed != games_at_last_print
    {
        on_print_results(&tournament_stats, &player_stats, settings);
    }
    if let Err(err) = save_results(
        &tournament_stats,
        &player_stats,
        &pair_store,
        &has_result,
        true,
    ) {
        println!("<Warning> {}", err);
    }

    println!();
    println!("Match Statistics:");
    println!("Games played: {}", tournament_stats.games_completed);
    println!("Players created: {}", tournament_stats.players_created);
    println!("Players destroyed: {}", tournament_stats.players_destroyed);
    if tournament_stats.games_completed > 0 {
        println!(
            "Average game length: {:.1} ply",
            tournament_stats.total_ply as f32 / tournament_stats.games_completed as f32
        );
    }

    if settings.verbose {
        println!();
        println!("Debug:");
        println!("Pair store: {}", pair_store.len());
    }

    println!();
    println!(
        "{:<3} {:<12} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4}",
        "id", "name", "w", "l", "d", "ww", "wl", "wd", "lw", "ll", "ld", "dw", "dl", "dd"
    );
    for (id, stats) in &player_stats {
        println!(
            "{:<3} {:<12} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4} {:>4}",
            id,
            stats.name,
            stats.wld.w,
            stats.wld.l,
            stats.wld.d,
            stats.wld_pairs.ww,
            stats.wld_pairs.wl,
            stats.wld_pairs.wd,
            stats.wld_pairs.lw,
            stats.wld_pairs.ll,
            stats.wld_pairs.ld,
            stats.wld_pairs.dw,
            stats.wld_pairs.dl,
            stats.wld_pairs.dd
        );
    }

    let any_aborts = player_stats
        .values()
        .any(|s| s.crashes > 0 || s.timeouts > 0 || s.illegalmoves > 0);
    if any_aborts {
        println!();
        println!(
            "{:<3} {:<12} {:>8} {:>8} {:>7}",
            "id", "name", "crashes", "timeouts", "illegal"
        );
        for (id, stats) in &player_stats {
            println!(
                "{:<3} {:<12} {:>8} {:>8} {:>7}",
                id, stats.name, stats.crashes, stats.timeouts, stats.illegalmoves
            );
        }
    }

    println!();
    println!("+2 +1 +0 -1 -2");
    for stats in player_stats.values() {
        let far_ahead = stats.wld_pairs.ww;
        let ahead = stats.wld_pairs.wd + stats.wld_pairs.dw;
        let even = stats.wld_pairs.wl + stats.wld_pairs.lw + stats.wld_pairs.dd;
        let behind = stats.wld_pairs.ld + stats.wld_pairs.dl;
        let far_behind = stats.wld_pairs.ll;

        println!(
            "{:>2} {:>2} {:>2} {:>2} {:>2} {}",
            far_ahead, ahead, even, behind, far_behind, stats.name
        );
    }

    Ok(())
}
