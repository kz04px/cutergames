use crate::players::{Player, clock::ClockType, process::EngineProcess};
use ataxx::{moves::Move, position::Position, result::GameResult, side::Side};

pub struct UAIEngine {
    name: String,
    process: EngineProcess,
    pos: Position,
    clock: ClockType,
}

impl UAIEngine {
    pub fn new(name: &str, path: &str, clock: ClockType, debug: bool) -> Self {
        if debug {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|line| {
                    println!("{:?}:uai> {}", std::thread::current().id(), line);
                }),
                pos: Position::from_fen("startpos"),
                clock,
            }
        } else {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|_line| {}),
                pos: Position::from_fen("startpos"),
                clock,
            }
        }
    }
}

impl Drop for UAIEngine {
    fn drop(&mut self) {
        self.process.send("stop\n");
        self.process.send("quit\n");
    }
}

impl Player for UAIEngine {
    fn init(&mut self) {
        self.process.send("uai\n");
        self.process.wait("uaiok\n");
    }

    fn isready(&mut self) {
        self.process.send("isready\n");
        self.process.wait("readyok\n");
    }

    fn set_position(&mut self, fen: &str) {
        if fen == "startpos" {
            self.pos = Position::from_fen("startpos");
            self.process.send("position startpos\n");
        } else {
            self.pos = Position::from_fen(fen);
            self.process.send(&format!("position fen {}\n", fen));
        }
    }

    fn get_move(&mut self) -> Option<String> {
        self.process.send("go movetime 10\n");
        let mut movestr = None;
        self.process.wait_magic(&mut |msg: &str| -> bool {
            let found_bestmove = msg[0..8] == *"bestmove";
            if found_bestmove {
                movestr = Some(msg[9..msg.len() - 1].to_string());
                true
            } else {
                false
            }
        });
        movestr
    }

    fn makemove(&mut self, mvstr: &str) -> bool {
        let mv = Move::from_string(mvstr);
        self.pos.makemove(&mv);
        self.process.send(&format!("moves {}\n", mvstr));
        true
    }

    fn is_gameover(&mut self) -> bool {
        self.pos.is_gameover()
    }

    fn is_legal(&mut self, mvstr: &str) -> bool {
        let mv = Move::from_string(mvstr);
        self.pos.is_legal_move(&mv)
    }

    fn get_turn(&mut self) -> Option<usize> {
        match self.pos.turn {
            Side::Black => Some(0),
            Side::White => Some(1),
        }
    }

    fn query_result(&mut self) -> Option<String> {
        match self.pos.get_result() {
            Some(GameResult::BlackWin) => Some("p1win".to_string()),
            Some(GameResult::WhiteWin) => Some("p2win".to_string()),
            Some(GameResult::Draw) => Some("draw".to_string()),
            None => None,
        }
    }

    fn get_clock(&mut self) -> &mut super::clock::ClockType {
        &mut self.clock
    }
}
