use crate::players::Player;
use crate::players::clock::ClockType;
use crate::tournament::GameData;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    Crash(usize),
}

// How a game-ending abort is attributed to a player for statistics.
#[derive(Debug, Clone, Copy)]
pub enum AbortKind {
    Timeout,
    IllegalMove,
    Crash,
}

#[must_use]
pub fn play(
    game_id: usize,
    p1_id: usize,
    p2_id: usize,
    fen: &str,
    players: &[Rc<RefCell<Box<dyn Player>>>],
) -> GameData {
    debug_assert!(players.len() > 1);

    let max_ply = 1024;
    let mut result = None;
    let mut num_ply = 0;
    let mut aborted = None;

    // Set initial positions
    for player in players {
        player.borrow_mut().set_position(fen);
    }

    loop {
        // Get our judge
        if players[0].borrow_mut().is_gameover() {
            break;
        }

        // Get current turn
        let turn = if let Some(turn) = players[0].borrow_mut().get_turn() {
            turn
        } else {
            aborted = Some(AbortReason::NoTurn);
            break;
        };

        // isready
        players[turn].borrow_mut().isready();

        // The go command always describes both sides' clocks, regardless
        // of whose turn it is
        let p1_clock = *players[0].borrow_mut().get_clock();
        let p2_clock = *players[1].borrow_mut().get_clock();

        let start = Instant::now();

        // Get move from current player
        let mv = match players[turn].borrow_mut().get_move(p1_clock, p2_clock) {
            Some(mv) => mv,
            None => {
                if turn == 0 {
                    result = Some(GameOutcome::P2win);
                } else {
                    result = Some(GameOutcome::P1win);
                }
                aborted = Some(AbortReason::Crash(turn));
                break;
            }
        };

        let elapsed = start.elapsed();

        // Legal move?
        let is_legal = players[0].borrow_mut().is_legal(&mv);

        // Ran out of time?
        let is_timeout = {
            let mut player = players[turn].borrow_mut();
            match player.get_clock() {
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
            }
        };

        // Abort - Illegal move
        if !is_legal {
            if turn == 0 {
                result = Some(GameOutcome::P2win);
            } else {
                result = Some(GameOutcome::P1win);
            }
            aborted = Some(AbortReason::IllegalMove(turn, mv));
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
        for player in players {
            player.borrow_mut().makemove(&mv);
        }
    }

    // A turn slot (0 or 1) maps onto p1_id/p2_id the same way outcomes do above.
    let player_id_of = |turn: usize| if turn == 0 { p1_id } else { p2_id };

    // Was the game aborted? The result, if any, was decided above by forfeit.
    if let Some(reason) = &aborted {
        let abort = match reason {
            AbortReason::Timeout(id) => {
                println!("<Abort> Game #{} -- player {} ran out of time", game_id, id);
                Some((player_id_of(*id), AbortKind::Timeout))
            }
            AbortReason::IllegalMove(id, movestr) => {
                println!(
                    "<Abort> Game #{} -- player {} played the illegal move \"{}\"",
                    game_id, id, movestr
                );
                Some((player_id_of(*id), AbortKind::IllegalMove))
            }
            AbortReason::MaxGameLength => {
                println!("<Abort> Game #{} -- maximum game length reached", game_id);
                None
            }
            AbortReason::NoTurn => {
                println!(
                    "<Abort> Game #{} -- couldn't determine whose turn it is",
                    game_id
                );
                None
            }
            AbortReason::Crash(id) => {
                println!(
                    "<Abort> Game #{} -- player {} stopped responding",
                    game_id, id
                );
                Some((player_id_of(*id), AbortKind::Crash))
            }
        };

        return GameData {
            id: game_id,
            outcome: result,
            player1: p1_id,
            player2: p2_id,
            ply: num_ply,
            abort,
        };
    }

    // Ask the judge for the result
    result = match players[0].borrow_mut().query_result().as_deref() {
        Some("p1win") => Some(GameOutcome::P1win),
        Some("p2win") => Some(GameOutcome::P2win),
        Some("draw") => Some(GameOutcome::Draw),
        _ => None,
    };

    GameData {
        id: game_id,
        outcome: result,
        player1: p1_id,
        player2: p2_id,
        ply: num_ply,
        abort: None,
    }
}
