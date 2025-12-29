use crate::players::{Player, engine::EngineProcess};

pub struct UAIEngine {
    name: String,
    process: EngineProcess,
}

impl UAIEngine {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            process: EngineProcess::new(path),
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
                return true;
            } else {
                return false;
            }
        });
        movestr
    }

    fn makemove(&mut self, mvstr: &str) -> bool {
        self.process.send(&format!("moves {}\n", mvstr));
        true
    }

    fn is_gameover(&mut self) -> bool {
        false
    }

    fn is_legal(&mut self, _mvstr: &str) -> bool {
        true
    }

    fn get_turn(&mut self) -> Option<usize> {
        Some(0)
    }

    fn query_result(&mut self) -> Option<String> {
        Some("false".to_string())
    }
}
