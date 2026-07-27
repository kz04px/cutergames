use crate::players::clock::{ClockType, go_command};
use crate::players::{Player, process::EngineProcess};

pub struct UGIEngine {
    process: EngineProcess,
    clock: ClockType,
}

impl UGIEngine {
    pub fn new(name: &str, path: &str, parameters: &str, clock: ClockType, debug: bool) -> Self {
        let name = name.to_string();
        let callback: Box<dyn Fn(&str)> = if debug {
            Box::new(move |line: &str| println!("{}:ugi> {}", name, line))
        } else {
            Box::new(|_line: &str| {})
        };

        Self {
            process: EngineProcess::new(path, parameters, callback),
            clock,
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

    fn set_option(&mut self, name: &str, value: &str) {
        self.process.set_option(name, value);
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

    fn get_move(&mut self, p1_clock: ClockType, p2_clock: ClockType) -> Option<String> {
        self.process
            .send(&go_command(self.clock, p1_clock, p2_clock));
        let mut movestr = None;
        self.process.wait_magic(&mut |msg: &str| -> bool {
            let mut parts = msg.split_whitespace();
            if parts.next() != Some("bestmove") {
                return false;
            }
            movestr = parts.next().map(String::from);
            true
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
            if msg.trim_end() == "response true" {
                is_over = true;
                true
            } else if msg.trim_end() == "response false" {
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
            if msg.trim_end() == "response true" {
                turn = Some(0);
                true
            } else if msg.trim_end() == "response false" {
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
            match msg.trim_end() {
                "response p1win" => {
                    result = Some("p1win".to_string());
                    true
                }
                "response p2win" => {
                    result = Some("p2win".to_string());
                    true
                }
                "response draw" => {
                    result = Some("draw".to_string());
                    true
                }
                "response none" => {
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
