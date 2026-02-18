use crate::players::clock::ClockType;
use crate::players::{Player, process::EngineProcess};

pub struct UGIEngine {
    name: String,
    process: EngineProcess,
    clock: ClockType,
}

impl UGIEngine {
    pub fn new(name: &str, path: &str, clock: ClockType, debug: bool) -> Self {
        if debug {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|line| {
                    println!("{:?}:ugi> {}", std::thread::current().id(), line);
                }),
                clock,
            }
        } else {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|_line| {}),
                clock,
            }
        }
    }
}

impl Drop for UGIEngine {
    fn drop(&mut self) {
        self.process.send("stop\n");
        self.process.send("quit\n");
    }
}

impl Player for UGIEngine {
    fn init(&mut self) {
        self.process.send("ugi\n");
        self.process.wait("ugiok\n");
    }

    fn isready(&mut self) {
        self.process.send("isready\n");
        self.process.wait("readyok\n");
    }

    fn set_position(&mut self, fen: &str) {
        if fen == "startpos" {
            self.process.send("position startpos\n");
        } else {
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
        self.process.send(&format!("moves {}\n", mvstr));
        true
    }

    fn is_gameover(&mut self) -> bool {
        self.process.send("query gameover\n");
        let mut is_over = false;
        self.process.wait_magic(&mut |msg: &str| -> bool {
            if msg == "response true\n" {
                is_over = true;
                true
            } else if msg == "response false\n" {
                is_over = false;
                true
            } else {
                false
            }
        });
        is_over
    }

    fn is_legal(&mut self, _mvstr: &str) -> bool {
        true
    }

    fn get_turn(&mut self) -> Option<usize> {
        self.process.send("query p1turn\n");
        let mut turn = None;
        self.process.wait_magic(&mut |msg: &str| -> bool {
            if msg == "response true\n" {
                turn = Some(0);
                true
            } else if msg == "response false\n" {
                turn = Some(1);
                true
            } else {
                false
            }
        });
        turn
    }

    fn query_result(&mut self) -> Option<String> {
        self.process.send("query result\n");
        let mut result = None;
        self.process.wait_magic(&mut |msg: &str| -> bool {
            match msg {
                "response p1win\n" => {
                    result = Some("p1win".to_string());
                    true
                }
                "response p2win\n" => {
                    result = Some("p2win".to_string());
                    true
                }
                "response draw\n" => {
                    result = Some("draw".to_string());
                    true
                }
                "response none\n" => {
                    result = Some("none".to_string());
                    true
                }
                _ => false,
            }
        });
        result
    }

    fn get_clock(&mut self) -> &mut ClockType {
        &mut self.clock
    }
}
