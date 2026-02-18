use crate::players::Player;
use crate::players::clock::ClockType;
use crate::tournament::GameData;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum GameOutcome {
    P1win,
    P2win,
    Draw,
}

enum AbortReason {
    Timeout(usize),
    IllegalMove(usize, String),
    MaxGameLength,
    NoTurn,
    PlayerStall,
}

#[must_use]
pub fn play(
    game_id: usize,
    p1_id: usize,
    p2_id: usize,
    fen: &str,
    players: &mut Vec<Box<dyn Player>>,
) -> Option<GameData> {
    debug_assert!(players.len() > 1);

    let max_ply = 1024;
    let mut result = None;
    let mut num_ply = 0;
    let mut aborted = None;

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
        let turn = if let Some(turn) = players.get_mut(0).expect("Couldn't find judge").get_turn() {
            turn
        } else {
            aborted = Some(AbortReason::NoTurn);
            break;
        };

        // isready
        players
            .get_mut(turn)
            .expect("Can't find current player")
            .isready();

        let start = Instant::now();

        // Get move from current player
        let mv = players
            .get_mut(turn)
            .expect("Can't find current player")
            .get_move()
            .expect("No move string returned from engine");

        let elapsed = start.elapsed();

        // Legal move?
        let is_legal = players
            .get_mut(0)
            .expect("Couldn't find judge")
            .is_legal(&mv);

        let clock = players
            .get_mut(turn)
            .expect("Can't find current player")
            .get_clock();

        // Ran out of time?
        let is_timeout = match clock {
            ClockType::Movetime(ms) => elapsed.as_millis() > *ms + 10,
            ClockType::Depth(_) => false,
            ClockType::Time(remaining, increment) => {
                if elapsed.as_millis() >= *remaining {
                    true
                } else {
                    *remaining -= elapsed.as_millis();
                    *remaining += *increment;
                    false
                }
            }
        };

        // Abort - Illegal move
        if !is_legal {
            if turn == 0 {
                result = Some(GameOutcome::P2win);
            } else {
                result = Some(GameOutcome::P1win);
            }
            aborted = Some(AbortReason::IllegalMove(0, mv));
            break;
        }

        // Abort - Out of time
        if is_timeout {
            if turn == 0 {
                result = Some(GameOutcome::P2win);
            } else {
                result = Some(GameOutcome::P1win);
            }
            aborted = Some(AbortReason::Timeout(turn));
            break;
        }

        // Abort - Max game length reached
        if num_ply >= max_ply {
            result = Some(GameOutcome::Draw);
            aborted = Some(AbortReason::MaxGameLength);
            break;
        }

        num_ply += 1;

        // Update positions
        for player in players.iter_mut() {
            player.makemove(&mv);
        }
    }

    // Was the game aborted?
    match aborted {
        // Result exists due to forfeit
        Some(AbortReason::Timeout(_id)) => {
            return Some(GameData {
                id: 0,
                outcome: result,
                player1: 0,
                player2: 1,
                ply: num_ply,
            });
        }
        Some(AbortReason::IllegalMove(_id, _movestr)) => {
            return Some(GameData {
                id: 0,
                outcome: result,
                player1: 0,
                player2: 1,
                ply: num_ply,
            });
        }
        Some(AbortReason::PlayerStall) => {
            return Some(GameData {
                id: 0,
                outcome: result,
                player1: 0,
                player2: 1,
                ply: num_ply,
            });
        }
        Some(AbortReason::MaxGameLength) => {
            return Some(GameData {
                id: 0,
                outcome: None,
                player1: 0,
                player2: 1,
                ply: num_ply,
            });
        }
        // No result
        Some(AbortReason::NoTurn) => {
            return Some(GameData {
                id: 0,
                outcome: None,
                player1: 0,
                player2: 1,
                ply: num_ply,
            });
        }
        None => {}
    }

    // Ask the judge for the result
    let asd = players
        .get_mut(0)
        .expect("Couldn't find judge")
        .query_result()
        .unwrap();
    result = match asd.as_str() {
        "p1win" => Some(GameOutcome::P1win),
        "p2win" => Some(GameOutcome::P2win),
        "draw" => Some(GameOutcome::Draw),
        _ => None,
    };

    Some(GameData {
        id: game_id,
        outcome: result,
        player1: p1_id,
        player2: p2_id,
        ply: num_ply,
    })
}
