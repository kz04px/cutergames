use crate::game::{Game, GamePosition};
use crate::players::{Player, clock::ClockType, process::EngineProcess};

// The only Tak board CuterGames plays, with no komi
const TAK_SIZE: i32 = 6;
const TAK_HALFKOMI: i32 = 0;

pub struct TEIEngine {
    process: EngineProcess,
    game: Game,
    pos: GamePosition,
    base_tps: String,
    moves: Vec<String>,
    clock: ClockType,
}

impl TEIEngine {
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
            Box::new(move |line: &str| println!("{}:tei> {}", name, line))
        } else {
            Box::new(|_line: &str| {})
        };
        let process = EngineProcess::new(path, parameters, callback);

        Self {
            process,
            game,
            pos: GamePosition::new(game, "startpos"),
            base_tps: "startpos".to_string(),
            moves: Vec::new(),
            clock,
        }
    }

    // Like UCI, TEI has no way to ask an engine about a position, so the
    // full move list is resent as part of the `position` command every time.
    fn position_command(&self) -> String {
        let mut cmd = if self.base_tps == "startpos" {
            "position startpos".to_string()
        } else {
            format!("position tps {}", self.base_tps)
        };

        if !self.moves.is_empty() {
            cmd.push_str(" moves ");
            cmd.push_str(&self.moves.join(" "));
        }

        cmd.push('\n');
        cmd
    }

    // TEI's go command uses wtime/btime rather than the p1time/p2time of
    // UGI/UAI, since it's always relative to White and Black.
    fn go_command(&self, p1_clock: ClockType, p2_clock: ClockType) -> String {
        // Movetime and depth are the mover's own limits, not player 1's
        match self.clock {
            ClockType::Movetime(ms) => format!("go movetime {}\n", ms),
            ClockType::Depth(depth) => format!("go depth {}\n", depth),
            ClockType::Time(..) => {
                let (ClockType::Time(p1_time, p1_inc), ClockType::Time(p2_time, p2_inc)) =
                    (p1_clock, p2_clock)
                else {
                    unreachable!("Both players must be using the same clock type");
                };
                format!(
                    "go wtime {} btime {} winc {} binc {}\n",
                    p1_time, p2_time, p1_inc, p2_inc
                )
            }
        }
    }
}

impl Drop for TEIEngine {
    fn drop(&mut self) {
        self.process.send("quit\n");
    }
}

impl Player for TEIEngine {
    fn init(&mut self) {
        self.process.send("tei\n");
        self.process.wait("teiok\n");
    }

    fn set_option(&mut self, name: &str, value: &str) {
        self.process.set_option(name, value);
    }

    fn isready(&mut self) {
        self.process.send("isready\n");
        self.process.wait("readyok\n");
    }

    fn set_position(&mut self, tps: &str) {
        self.process.send(&format!(
            "teinewgame size {} halfkomi {}\n",
            TAK_SIZE, TAK_HALFKOMI
        ));
        self.pos = GamePosition::new(self.game, tps);
        self.base_tps = tps.to_string();
        self.moves.clear();
    }

    fn get_move(&mut self, p1_clock: ClockType, p2_clock: ClockType) -> Option<String> {
        let position_cmd = self.position_command();
        self.process.send(&position_cmd);

        let go_cmd = self.go_command(p1_clock, p2_clock);
        self.process.send(&go_cmd);

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
        let applied = self.pos.makemove(mvstr);
        self.moves.push(mvstr.to_string());
        applied
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

    fn get_clock(&mut self) -> &mut ClockType {
        &mut self.clock
    }
}
