use crate::players::{Player, process::EngineProcess};

pub struct UCIEngine {
    name: String,
    process: EngineProcess,
    // game: Chess,
}

impl UCIEngine {
    pub fn new(name: &str, path: &str, debug: bool) -> Self {
        if debug {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|line| {
                    println!("{}", line);
                }),
            }
        } else {
            Self {
                name: name.to_string(),
                process: EngineProcess::new(path, &|_line| {}),
            }
        }
    }
}

impl Player for UCIEngine {
    fn init(&mut self) {
        todo!()
    }

    fn isready(&mut self) {
        todo!()
    }

    fn set_position(&mut self, _fen: &str) {
        todo!()
    }

    fn get_move(&mut self) -> Option<String> {
        todo!()
    }

    fn makemove(&mut self, _mvstr: &str) -> bool {
        todo!()
    }

    fn is_gameover(&mut self) -> bool {
        todo!()
    }

    fn is_legal(&mut self, _mvstr: &str) -> bool {
        todo!()
    }

    fn get_turn(&mut self) -> Option<usize> {
        todo!()
    }

    fn query_result(&mut self) -> Option<String> {
        todo!()
    }

    fn get_clock(&mut self) -> &mut super::clock::ClockType {
        todo!()
    }
}
