use crate::game::{Game, GamePosition};
use crate::players::{
    Player,
    clock::{ClockType, go_command},
    process::EngineProcess,
};

pub struct UAIEngine {
    process: EngineProcess,
    game: Game,
    pos: GamePosition,
    clock: ClockType,
}

impl UAIEngine {
    pub fn new(
        name: &str,
        path: &str,
        parameters: &str,
        game: Game,
        clock: ClockType,
        debug: bool,
    ) -> Self {
        let name = name.to_string();
        let callback: Box<dyn Fn(&str)> = if debug {
            Box::new(move |line: &str| println!("{}:uai> {}", name, line))
        } else {
            Box::new(|_line: &str| {})
        };

        Self {
            process: EngineProcess::new(path, parameters, callback),
            game,
            pos: GamePosition::new(game, "startpos"),
            clock,
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

    fn set_option(&mut self, name: &str, value: &str) {
        self.process.set_option(name, value);
    }

    fn isready(&mut self) {
        self.process.send("isready\n");
        self.process.wait("readyok\n");
    }

    fn set_position(&mut self, fen: &str) {
        self.pos = GamePosition::new(self.game, fen);
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
        self.pos.makemove(mvstr);
        self.process.send(&format!("moves {}\n", mvstr));
        true
    }

    fn is_gameover(&mut self) -> bool {
        self.pos.is_gameover()
    }

    fn is_legal(&mut self, mvstr: &str) -> bool {
        self.pos.is_legal(mvstr)
    }

    fn get_turn(&mut self) -> Option<usize> {
        self.pos.get_turn()
    }

    fn query_result(&mut self) -> Option<String> {
        self.pos.query_result()
    }

    fn get_clock(&mut self) -> &mut super::clock::ClockType {
        &mut self.clock
    }
}
