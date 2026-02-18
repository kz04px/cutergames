use crate::{
    players::{self, Player},
    tournament::{Event, TournamentSettings, generator::Generator, play::play},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

pub fn worker(
    generator: Arc<Mutex<Generator>>,
    worker_id: usize,
    send: Sender<Event>,
    settings: &TournamentSettings,
    should_stop: Arc<AtomicBool>,
) {
    let _ = send.send(Event::ThreadStart(worker_id));

    while !should_stop.load(Ordering::Relaxed) {
        let work = if let Some(work) = generator.lock().expect("Lock fail thingy").next() {
            work
        } else {
            break;
        };

        // Create players
        let mut players: Vec<Box<dyn Player>> = vec![
            players::create(
                &settings.players.get(work.player1).unwrap().name,
                &settings.players.get(work.player1).unwrap().path,
                players::get_protocol(&settings.players.get(work.player1).unwrap().proto).unwrap(),
                settings.players.get(work.player1).unwrap().debug,
            ),
            players::create(
                &settings.players.get(work.player2).unwrap().name,
                &settings.players.get(work.player2).unwrap().path,
                players::get_protocol(&settings.players.get(work.player2).unwrap().proto).unwrap(),
                settings.players.get(work.player2).unwrap().debug,
            ),
        ];

        let _ = send.send(Event::PlayerCreate(work.player1));
        let _ = send.send(Event::PlayerCreate(work.player2));

        // Initialise players
        for player in players.iter_mut() {
            player.init();
            player.isready();
        }

        let _ = send.send(Event::GameStart(work.game_id, work.fen_idx));
        let result = play(
            work.game_id,
            work.player1,
            work.player2,
            &settings.fens[work.fen_idx],
            &mut players,
        );

        if let Some(data) = result {
            let _ = send.send(Event::GameFinish(data));
        } else {
            println!("Uh oh");
        }

        for _ in players {
            let _ = send.send(Event::PlayerDestroy);
        }
    }

    let _ = send.send(Event::ThreadFinish(worker_id));
}
