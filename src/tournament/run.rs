use crate::tournament::on_game_finish::on_game_finish;
use crate::tournament::on_game_start::on_game_start;
use crate::tournament::on_player_create::on_player_create;
use crate::tournament::on_player_destroy::on_player_destroy;
use crate::tournament::on_thread_finish::on_thread_finish;
use crate::tournament::on_thread_start::on_thread_start;
use crate::tournament::worker::worker;
use crate::tournament::{Event, PlayerStatistics, TournamentSettings, TournamentStatistics};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, sync::mpsc::channel, thread};

pub fn run(settings: &TournamentSettings) {
    let (send, recv) = channel();
    let mut threads = vec![];
    let mut player_stats: HashMap<usize, PlayerStatistics> = HashMap::new();
    let should_stop = Arc::new(AtomicBool::new(false));

    // Initialise player statistics
    for idx in 0..settings.players.len() {
        player_stats.insert(
            idx,
            PlayerStatistics {
                name: settings.players.iter().nth(idx).unwrap().name.clone(),
                played: 0,
                crashes: 0,
                wins: 0,
                draws: 0,
                losses: 0,
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

    // Worker threads
    for thread_id in 0..settings.num_threads {
        let set = settings.clone();
        let nsend = send.clone();
        let asd = should_stop.clone();
        let thread = thread::spawn(move || worker(thread_id, nsend, &set, asd));
        threads.push(thread);
    }

    // Statistics
    let mut tournament_stats = TournamentStatistics::default();

    loop {
        match recv.recv() {
            // Game
            Ok(Event::GameStart(id)) => on_game_start(id, settings.verbose),
            Ok(Event::GameFinish(data)) => on_game_finish(
                send.clone(),
                &data,
                &mut player_stats,
                &mut tournament_stats,
                &settings,
            ),
            // Tournament
            Ok(Event::TournamentStart) => {}
            Ok(Event::TournamentFinish) => {
                should_stop.store(true, Ordering::Relaxed);
                break;
            }
            // Players
            Ok(Event::PlayerCreate(id)) => {
                on_player_create(id, &mut tournament_stats, settings.verbose);
            }
            Ok(Event::PlayerDestroy) => on_player_destroy(&mut tournament_stats, settings.verbose),
            // Threads
            Ok(Event::ThreadStart(id)) => on_thread_start(id, settings.verbose),
            Ok(Event::ThreadFinish(id)) => on_thread_finish(id, settings.verbose),
            // Other
            Ok(Event::KeyPress) => println!("keypress"),
            Err(_e) => break,
        }
    }

    debug_assert!(should_stop.load(Ordering::Relaxed));

    for thread in threads {
        let _ = thread.join();
    }

    println!("");
    println!("Games played: {}", tournament_stats.games_completed);
    println!("Players created: {}", tournament_stats.players_created);
    println!("Players destroyed: {}", tournament_stats.players_destroyed);
}
