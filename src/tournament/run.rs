use crate::stats::{WLD, WLDPairs};
use crate::tournament::generator::Generator;
use crate::tournament::on_game_finish::on_game_finish;
use crate::tournament::on_game_pair_result::on_game_pair_result;
use crate::tournament::on_game_start::on_game_start;
use crate::tournament::on_player_create::on_player_create;
use crate::tournament::on_player_destroy::on_player_destroy;
use crate::tournament::on_print_results::on_print_results;
use crate::tournament::on_thread_finish::on_thread_finish;
use crate::tournament::on_thread_start::on_thread_start;
use crate::tournament::play::GameOutcome;
use crate::tournament::worker::worker;
use crate::tournament::{Event, PlayerStatistics, TournamentSettings, TournamentStatistics};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, sync::mpsc::channel, thread};

pub fn run(settings: &TournamentSettings) {
    let (send, recv) = channel();
    let mut threads = vec![];
    let mut player_stats: HashMap<usize, PlayerStatistics> = HashMap::new();
    let should_stop = Arc::new(AtomicBool::new(false));
    let mut pair_store: HashMap<usize, GameOutcome> = HashMap::new();
    let mut threads_running = 0;

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

    // Keypress thread
    let nsend = send.clone();
    thread::spawn(move || {
        loop {
            let mut buffer = String::new();
            let _ = std::io::stdin().read_line(&mut buffer);
            let _ = nsend.send(Event::KeyPress);
        }
    });

    // Work generator
    let generator = Arc::new(Mutex::new(Generator::new(
        settings.players.len(),
        settings.fens.len(),
        settings.num_games,
    )));

    // Worker threads
    for thread_id in 0..settings.num_threads {
        let set = settings.clone();
        let nsend = send.clone();
        let asd = should_stop.clone();
        let dsa = generator.clone();
        let thread = thread::spawn(move || worker(dsa, thread_id, nsend, &set, asd));
        threads.push(thread);
    }

    // Statistics
    let mut tournament_stats = TournamentStatistics::default();

    loop {
        match recv.recv() {
            // Game
            Ok(Event::GameStart(id, fen_idx)) => {
                on_game_start(id, &settings.fens[fen_idx], settings.verbose)
            }
            Ok(Event::GameFinish(data)) => on_game_finish(
                send.clone(),
                &data,
                &mut pair_store,
                &mut player_stats,
                &mut tournament_stats,
                settings,
            ),
            Ok(Event::GamePairResult(p1, p2, r1, r2)) => {
                on_game_pair_result(settings, &mut player_stats, p1, p2, r1, r2)
            }
            // Tournament
            Ok(Event::TournamentStart) => {}
            Ok(Event::TournamentFinish) => should_stop.store(true, Ordering::Relaxed),
            // Players
            Ok(Event::PlayerCreate(id)) => {
                on_player_create(id, &mut tournament_stats, settings.verbose)
            }
            Ok(Event::PlayerDestroy) => on_player_destroy(&mut tournament_stats, settings.verbose),
            // Threads
            Ok(Event::ThreadStart(id)) => {
                threads_running += 1;
                on_thread_start(id, settings.verbose);
            }
            Ok(Event::ThreadFinish(id)) => {
                threads_running -= 1;
                on_thread_finish(id, settings.verbose);

                if threads_running == 0 {
                    break;
                }
            }
            // Results
            Ok(Event::PrintUpdate) => on_print_results(&tournament_stats, &player_stats, settings),
            // Other
            Ok(Event::KeyPress) => println!("keypress"),
            Err(_e) => break,
        }
    }

    debug_assert_eq!(threads_running, 0);
    debug_assert!(recv.try_recv().is_err());

    for thread in threads {
        let _ = thread.join();
    }

    println!();
    println!("Match Statistics:");
    println!("Games played: {}", tournament_stats.games_completed);
    println!("Players created: {}", tournament_stats.players_created);
    println!("Players destroyed: {}", tournament_stats.players_destroyed);

    if true {
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
}
