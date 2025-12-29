use std::time::{Instant, SystemTime};

use crate::players::Player;

pub struct TimeControl {
    pub wtime: Option<u32>,
    pub btime: Option<u32>,
    pub movetime: Option<u32>,
}

pub struct PlaySettings {
    pub fen: String,
    pub tc: TimeControl,
}

#[derive(Debug)]
pub struct GameResult {
    pub outcome: String,
    pub ply: usize,
}

#[must_use]
pub fn play(players: &mut Vec<Box<dyn Player>>) -> Option<GameResult> {
    debug_assert!(players.len() > 1);

    let fen = "startpos";

    // Set initial positions
    for player in players.iter_mut() {
        player.set_position(fen);
    }

    loop {
        // Get our judge
        if players
            .get_mut(0)
            .expect("Couldn't find judge")
            .is_gameover()
        {
            break;
        }

        // Get current turn
        let turn = players
            .get_mut(0)
            .expect("Couldn't find judge")
            .get_turn()
            .expect("No player with turn");

        // isready
        players
            .get_mut(turn)
            .expect("Can't find current player")
            .isready();

        let _start = Instant::now();

        // Get move from current player
        let mv = players
            .get_mut(turn)
            .expect("Can't find current player")
            .get_move()
            .expect("No move string returned from engine");

        let _end = Instant::now();

        // Check move legality
        if !players
            .get_mut(0)
            .expect("Couldn't find judge")
            .is_legal(&mv)
        {
            break;
        }

        // Update positions
        for player in players.iter_mut() {
            player.makemove(&mv);
        }
    }

    let is_gameover = players
        .get_mut(0)
        .expect("Couldn't find judge")
        .is_gameover();
    let turn = players.get_mut(0).expect("Couldn't find judge").get_turn();
    let result = players
        .get_mut(0)
        .expect("Couldn't find judge")
        .query_result();

    // println!("Finished game");
    // println!("fen: {}", fen);
    // println!("Gameover: {}", is_gameover);
    // println!("Turn: {:?}", turn);
    // println!("Result: {:?}", result);

    Some(GameResult {
        outcome: result.unwrap(),
        ply: 0,
    })
}
