mod on_game_finish;
mod on_game_start;
mod on_player_create;
mod on_player_destroy;
mod on_thread_finish;
mod on_thread_start;
mod play;
pub mod run;
mod worker;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct PlayerSettings {
    pub name: String,
    pub path: String,
    pub proto: String,
    pub parameters: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TournamentSettings {
    pub players: Vec<PlayerSettings>,
    pub fens: Vec<String>,
    pub num_threads: usize,
    pub num_games: Option<usize>,
    pub sprt_trinomial: bool,
    pub sprt_pentanomial: bool,
    pub verbose: bool,
}

#[derive(Default)]
struct PlayerStatistics {
    name: String,
    played: usize,
    crashes: usize,
    // Trinomial
    wins: usize,
    draws: usize,
    losses: usize,
    // Pentanomial
    // ww: usize,
    // wl: usize,
    // wd: usize,
    // lw: usize,
    // ll: usize,
    // ld: usize,
    // dw: usize,
    // dl: usize,
    // dd: usize,
}

#[derive(Default)]
struct TournamentStatistics {
    games_completed: usize,
    players_created: usize,
    players_destroyed: usize,
}

struct GameResult {
    id: usize,
    outcome: String,
    player1: usize,
    player2: usize,
    ply: usize,
}

enum Event {
    // Game
    GameStart(usize),
    GameFinish(GameResult),
    // Tournament
    TournamentStart,
    TournamentFinish,
    // Players
    PlayerCreate(usize),
    PlayerDestroy,
    // Threads
    ThreadStart(usize),
    ThreadFinish(usize),
    // Other
    KeyPress,
}

impl Default for TournamentSettings {
    fn default() -> Self {
        Self {
            players: vec![],
            fens: vec![],
            num_threads: 1,
            num_games: None,
            sprt_trinomial: false,
            sprt_pentanomial: false,
            verbose: false,
        }
    }
}
