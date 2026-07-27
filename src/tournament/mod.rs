use crate::{
    game::Game,
    players::clock::ClockType,
    sprt::Model,
    stats::{WLD, WLDPairs},
    tournament::play::{AbortKind, GameOutcome},
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
mod results_file;
pub mod run;
mod worker;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct PlayerSettings {
    pub name: String,
    pub path: String,
    pub proto: String,
    pub parameters: String,
    // Engine options sent with `setoption` after the handshake, in the
    // order they were given
    pub options: Vec<(String, String)>,
    pub debug: bool,
    // None uses the tournament's clock
    pub clock: Option<ClockType>,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct SPRTSettings {
    pub alpha: f32,
    pub beta: f32,
    pub elo0: f32,
    pub elo1: f32,
    pub autostop: bool,
    // What elo0 and elo1 mean; see sprt::Model
    pub model: Model,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TournamentSettings {
    pub players: Vec<PlayerSettings>,
    pub fens: Vec<String>,
    // Where the fens were read from
    pub openings: Option<String>,
    // Where to save the settings and results as the match runs
    pub results: Option<String>,
    // Continue the unfinished match saved in `results`
    pub resume: bool,
    pub game: Game,
    pub num_threads: usize,
    pub num_games: Option<usize>,
    pub update_frequency: usize,
    pub sprt_trinomial: Option<SPRTSettings>,
    pub sprt_pentanomial: Option<SPRTSettings>,
    pub verbose: bool,
    // The time control for players that don't set their own
    pub clock: ClockType,
}

impl TournamentSettings {
    pub fn player_clock(&self, player: &PlayerSettings) -> ClockType {
        player.clock.unwrap_or(self.clock)
    }
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
    total_ply: usize,
}

struct GameData {
    id: usize,
    outcome: Option<GameOutcome>,
    player1: usize,
    player2: usize,
    ply: usize,
    // Which player forced the game to end early, if any, and how.
    abort: Option<(usize, AbortKind)>,
}

enum Event {
    // Game
    GameStart(usize, usize),
    GameFinish(GameData),
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
            openings: None,
            results: None,
            resume: false,
            game: Game::Ataxx,
            num_threads: 1,
            num_games: None,
            update_frequency: 10,
            sprt_trinomial: None,
            sprt_pentanomial: None,
            verbose: false,
            clock: ClockType::default(),
        }
    }
}
