use crate::{
    players::{self, Player},
    tournament::{Event, GameResult, TournamentSettings, play::play},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

struct Work {
    player1: usize,
    player2: usize,
    fen_idx: usize,
}

#[must_use]
fn generator(game_id: usize, fens: &Vec<String>) -> Option<Work> {
    debug_assert!(!fens.is_empty());

    let player1 = game_id % 2;
    let player2 = 1 - player1;
    let fen_idx = (game_id / 2) % fens.len();

    debug_assert!(player1 != player2);
    debug_assert!(fen_idx < fens.len());

    Some(Work {
        player1,
        player2,
        fen_idx,
    })
}

pub fn worker(
    worker_id: usize,
    send: Sender<Event>,
    settings: &TournamentSettings,
    should_stop: Arc<AtomicBool>,
) {
    let _ = send.send(Event::ThreadStart(worker_id));
    let mut game_id = worker_id;
    let fens = vec!["startpos".to_string()];

    while !should_stop.load(Ordering::Relaxed) {
        let work = generator(game_id, &fens).unwrap();

        // Create players
        let mut players: Vec<Box<dyn Player>> = vec![
            players::create(
                &settings.players.iter().nth(work.player1).unwrap().name,
                &settings.players.iter().nth(work.player1).unwrap().path,
                players::get_protocol(&settings.players.iter().nth(work.player1).unwrap().proto)
                    .unwrap(),
            ),
            players::create(
                &settings.players.iter().nth(work.player2).unwrap().name,
                &settings.players.iter().nth(work.player2).unwrap().path,
                players::get_protocol(&settings.players.iter().nth(work.player2).unwrap().proto)
                    .unwrap(),
            ),
        ];

        let _ = send.send(Event::PlayerCreate(work.player1));
        let _ = send.send(Event::PlayerCreate(work.player2));

        // Initialise players
        for player in players.iter_mut() {
            player.init();
            player.isready();
        }

        let _ = send.send(Event::GameStart(game_id));
        let result = play(&mut players);

        if result.is_none() {
            break;
        }

        let r = result.unwrap();
        let e = Event::GameFinish(GameResult {
            id: game_id,
            outcome: r.outcome,
            player1: work.player1,
            player2: work.player2,
            ply: r.ply,
        });
        let _ = send.send(e);

        game_id += settings.num_threads;
    }

    let _ = send.send(Event::ThreadFinish(worker_id));
}
