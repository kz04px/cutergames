use crate::{
    stats::{WLD, WLDPairs},
    tournament::play::GameOutcome,
};

mod generator;
mod on_game_finish;
mod on_game_pair_result;
mod on_game_start;
mod on_player_create;
mod on_player_destroy;
mod on_print_results;
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
    pub debug: bool,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SPRTSettings {
    pub alpha: f32,
    pub beta: f32,
    pub elo0: f32,
    pub elo1: f32,
    pub autostop: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TournamentSettings {
    pub players: Vec<PlayerSettings>,
    pub fens: Vec<String>,
    pub num_threads: usize,
    pub num_games: Option<usize>,
    pub update_frequency: usize,
    pub sprt_trinomial: Option<SPRTSettings>,
    pub sprt_pentanomial: Option<SPRTSettings>,
    pub verbose: bool,
}

#[derive(Default)]
struct PlayerStatistics {
    name: String,
    played: usize,
    crashes: usize,
    timeouts: usize,
    illegalmoves: usize,
    wld: WLD,
    wld_pairs: WLDPairs,
}

#[derive(Default)]
struct TournamentStatistics {
    games_completed: usize,
    players_created: usize,
    players_destroyed: usize,
}

struct GameData {
    id: usize,
    outcome: Option<GameOutcome>,
    player1: usize,
    player2: usize,
    ply: usize,
}

enum Event {
    // Game
    GameStart(usize, usize),
    GameFinish(GameData),
    GamePairResult(usize, usize, Option<GameOutcome>, Option<GameOutcome>),
    // Tournament
    TournamentStart,
    TournamentFinish,
    // Players
    PlayerCreate(usize),
    PlayerDestroy,
    // Threads
    ThreadStart(usize),
    ThreadFinish(usize),
    // Results
    PrintUpdate,
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
            update_frequency: 10,
            sprt_trinomial: None,
            sprt_pentanomial: None,
            verbose: false,
        }
    }
}
