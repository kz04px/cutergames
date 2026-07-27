use crate::{
    players::{self, Player},
    tournament::{Event, TournamentSettings, generator::Generator, play::play},
};
use lru::LruCache;
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

struct FinishGuard {
    worker_id: usize,
    send: Sender<Event>,
}

impl Drop for FinishGuard {
    fn drop(&mut self) {
        let _ = self.send.send(Event::ThreadFinish(self.worker_id));
    }
}

// Spawning an engine process is expensive, so each worker keeps its own
// engines alive across games rather than respawning them every time.
fn get_or_create(
    engines: &mut LruCache<usize, Rc<RefCell<Box<dyn Player>>>>,
    idx: usize,
    settings: &TournamentSettings,
    send: &Sender<Event>,
) -> Rc<RefCell<Box<dyn Player>>> {
    let engine = engines
        .get_or_insert(idx, || {
            let p = settings
                .players
                .get(idx)
                .expect("Work refers to an unknown player");
            let mut player = players::create(
                &p.name,
                &p.path,
                &p.parameters,
                players::get_protocol(&p.proto).expect("Unknown protocol"),
                settings.game,
                settings.player_clock(p),
                p.debug,
            );
            player.init();
            for (name, value) in &p.options {
                player.set_option(name, value);
            }
            player.isready();

            let _ = send.send(Event::PlayerCreate(idx));

            Rc::new(RefCell::new(player))
        })
        .clone();

    debug_assert!(engines.contains(&idx));
    debug_assert!(engines.len() <= engines.cap().get());

    engine
}

pub fn worker(
    generator: Arc<Mutex<Generator>>,
    worker_id: usize,
    send: Sender<Event>,
    settings: &TournamentSettings,
    should_stop: Arc<AtomicBool>,
) {
    let _guard = FinishGuard {
        worker_id,
        send: send.clone(),
    };

    let _ = send.send(Event::ThreadStart(worker_id));

    let capacity = NonZeroUsize::new(settings.players.len())
        .expect("Player count should never be zero given validated settings");
    let mut engines: LruCache<usize, Rc<RefCell<Box<dyn Player>>>> = LruCache::new(capacity);

    while !should_stop.load(Ordering::Relaxed) {
        let work = if let Some(work) = generator.lock().expect("Generator lock poisoned").next() {
            work
        } else {
            break;
        };

        let p1 = get_or_create(&mut engines, work.player1, settings, &send);
        let p2 = get_or_create(&mut engines, work.player2, settings, &send);
        let players = [p1, p2];

        // Capacity covers every player, so fetching p2 must not have evicted p1.
        // Otherwise engines would silently be respawned every game.
        debug_assert!(engines.contains(&work.player1));
        debug_assert!(engines.contains(&work.player2));

        let _ = send.send(Event::GameStart(work.game_id, work.fen_idx));
        let data = play(
            work.game_id,
            work.player1,
            work.player2,
            &settings.fens[work.fen_idx],
            &players,
        );

        let _ = send.send(Event::GameFinish(data));
    }

    debug_assert!(engines.len() <= settings.players.len());

    // No more work for this thread's engines, so shut them down
    while engines.pop_lru().is_some() {
        let _ = send.send(Event::PlayerDestroy);
    }
}
