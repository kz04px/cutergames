use games::{
    ataxx::AtaxxPosition7x7,
    chess::ChessPosition,
    connect4::Connect4Position,
    droptaxx::DroptaxxPosition,
    gamerules::{GameResult, GameRules},
    general::side::Side,
    gomoku::GomokuPosition,
    isolation::IsolationPosition,
    pijersi::PijersiPosition,
    tak::TakPosition,
};

// Tak is generic over its board size; 6x6 is the one we play.
type Tak6x6 = TakPosition<6>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Game {
    Ataxx,
    Chess,
    Connect4,
    Droptaxx,
    Gomoku,
    Isolation,
    Pijersi,
    Tak,
}

impl Game {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ataxx => "ataxx",
            Self::Chess => "chess",
            Self::Connect4 => "connect4",
            Self::Droptaxx => "droptaxx",
            Self::Gomoku => "gomoku",
            Self::Isolation => "isolation",
            Self::Pijersi => "pijersi",
            Self::Tak => "tak",
        }
    }
}

#[must_use]
pub fn parse_game_name(name: &str) -> Option<Game> {
    match name.to_lowercase().as_str() {
        "ataxx" => Some(Game::Ataxx),
        "chess" => Some(Game::Chess),
        "connect4" => Some(Game::Connect4),
        "droptaxx" => Some(Game::Droptaxx),
        "gomoku" => Some(Game::Gomoku),
        "isolation" => Some(Game::Isolation),
        "pijersi" => Some(Game::Pijersi),
        "tak" => Some(Game::Tak),
        _ => None,
    }
}

pub enum GamePosition {
    Ataxx(AtaxxPosition7x7),
    Chess(ChessPosition),
    Connect4(Connect4Position),
    Droptaxx(DroptaxxPosition),
    Gomoku(GomokuPosition),
    Isolation(IsolationPosition),
    Pijersi(PijersiPosition),
    Tak(Tak6x6),
}

fn side_to_index(side: Side) -> usize {
    match side {
        Side::Player1 => 0,
        Side::Player2 => 1,
    }
}

fn is_chess_square(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 2 && matches!(bytes[0], b'a'..=b'h') && matches!(bytes[1], b'1'..=b'8')
}

fn is_chess_movestr(mvstr: &str) -> bool {
    mvstr.is_ascii()
        && (mvstr.len() == 4 || mvstr.len() == 5)
        && is_chess_square(&mvstr[0..2])
        && is_chess_square(&mvstr[2..4])
}

fn result_to_string(result: GameResult) -> String {
    match result {
        GameResult::Win(Side::Player1) => "p1win".to_string(),
        GameResult::Win(Side::Player2) => "p2win".to_string(),
        GameResult::Draw => "draw".to_string(),
    }
}

impl GamePosition {
    #[must_use]
    pub fn new(game: Game, fen: &str) -> Self {
        match game {
            Game::Ataxx => Self::Ataxx(AtaxxPosition7x7::from_fen(fen)),
            Game::Chess => Self::Chess(ChessPosition::from_fen(fen)),
            Game::Connect4 => Self::Connect4(Connect4Position::from_fen(fen)),
            Game::Droptaxx => Self::Droptaxx(DroptaxxPosition::from_fen(fen)),
            Game::Gomoku => Self::Gomoku(GomokuPosition::from_fen(fen)),
            Game::Isolation => Self::Isolation(IsolationPosition::from_fen(fen)),
            Game::Pijersi => Self::Pijersi(PijersiPosition::from_fen(fen)),
            Game::Tak => Self::Tak(Tak6x6::from_fen(fen)),
        }
    }

    #[must_use]
    pub fn is_gameover(&self) -> bool {
        match self {
            Self::Ataxx(pos) => pos.is_gameover(),
            Self::Chess(pos) => pos.is_gameover(),
            Self::Connect4(pos) => pos.is_gameover(),
            Self::Droptaxx(pos) => pos.is_gameover(),
            Self::Gomoku(pos) => pos.is_gameover(),
            Self::Isolation(pos) => pos.is_gameover(),
            Self::Pijersi(pos) => pos.is_gameover(),
            Self::Tak(pos) => pos.is_gameover(),
        }
    }

    #[must_use]
    pub fn get_turn(&self) -> Option<usize> {
        Some(side_to_index(match self {
            Self::Ataxx(pos) => pos.get_turn(),
            Self::Chess(pos) => pos.get_turn(),
            Self::Connect4(pos) => pos.get_turn(),
            Self::Droptaxx(pos) => pos.get_turn(),
            Self::Gomoku(pos) => pos.get_turn(),
            Self::Isolation(pos) => pos.get_turn(),
            Self::Pijersi(pos) => pos.get_turn(),
            Self::Tak(pos) => pos.get_turn(),
        }))
    }

    #[must_use]
    pub fn query_result(&self) -> Option<String> {
        let result = match self {
            Self::Ataxx(pos) => pos.get_result(),
            Self::Chess(pos) => pos.get_result(),
            Self::Connect4(pos) => pos.get_result(),
            Self::Droptaxx(pos) => pos.get_result(),
            Self::Gomoku(pos) => pos.get_result(),
            Self::Isolation(pos) => pos.get_result(),
            Self::Pijersi(pos) => pos.get_result(),
            Self::Tak(pos) => pos.get_result(),
        }?;
        Some(result_to_string(result))
    }

    #[must_use]
    pub fn is_legal(&self, mvstr: &str) -> bool {
        match self {
            Self::Ataxx(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            // ChessPosition::parse_movestr() panics instead of returning an
            // Err for some malformed square strings (e.g. the null move
            // "0000" some UCI engines report when they have no legal
            // moves), so reject anything that isn't shaped like a chess
            // move before ever calling into it.
            Self::Chess(pos) => {
                if !is_chess_movestr(mvstr) {
                    println!("Failed to parse chess move string {:?}", mvstr);
                    false
                } else {
                    match pos.parse_movestr(mvstr) {
                        Ok(mv) => pos.legal_moves().contains(&mv),
                        Err(err) => {
                            println!("Failed to parse chess move string {:?}: {}", mvstr, err);
                            false
                        }
                    }
                }
            }
            Self::Connect4(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            Self::Droptaxx(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            Self::Gomoku(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            Self::Isolation(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            Self::Pijersi(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
            Self::Tak(pos) => pos
                .parse_move(mvstr)
                .is_ok_and(|mv| pos.legal_moves().contains(&mv)),
        }
    }

    /// Applies `mvstr` if (and only if) it's legal, reporting whether it was applied.
    pub fn makemove(&mut self, mvstr: &str) -> bool {
        if !self.is_legal(mvstr) {
            return false;
        }

        match self {
            Self::Ataxx(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Chess(pos) => {
                let mv = pos.parse_movestr(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Connect4(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Droptaxx(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Gomoku(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Isolation(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Pijersi(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
            Self::Tak(pos) => {
                let mv = pos.parse_move(mvstr).unwrap();
                pos.makemove(&mv);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_names() {
        assert_eq!(parse_game_name("ataxx"), Some(Game::Ataxx));
        assert_eq!(parse_game_name("Chess"), Some(Game::Chess));
        assert_eq!(parse_game_name("CONNECT4"), Some(Game::Connect4));
        assert_eq!(parse_game_name("droptaxx"), Some(Game::Droptaxx));
        assert_eq!(parse_game_name("gomoku"), Some(Game::Gomoku));
        assert_eq!(parse_game_name("isolation"), Some(Game::Isolation));
        assert_eq!(parse_game_name("tak"), Some(Game::Tak));
        assert_eq!(parse_game_name("Pijersi"), Some(Game::Pijersi));
        assert_eq!(parse_game_name("nonsense"), None);
    }

    #[test]
    fn chess_null_move_is_illegal_not_a_panic() {
        let pos = GamePosition::new(Game::Chess, "startpos");
        assert!(!pos.is_legal("0000"));
        assert!(!pos.is_legal(""));
        assert!(!pos.is_legal("e2e"));
        assert!(!pos.is_legal("e2e4e4e4"));
        assert!(pos.is_legal("e2e4"));
    }

    #[test]
    fn ataxx_startpos() {
        let mut pos = GamePosition::new(Game::Ataxx, "startpos");
        assert!(!pos.is_gameover());
        assert_eq!(pos.get_turn(), Some(0));
        assert!(pos.is_legal("f1"));
        assert!(!pos.is_legal("z9z9"));
        assert!(pos.makemove("f1"));
        assert_eq!(pos.get_turn(), Some(1));
    }

    #[test]
    fn tak_and_pijersi_startpos() {
        for game in [Game::Tak, Game::Pijersi] {
            let pos = GamePosition::new(game, "startpos");
            assert!(!pos.is_gameover());
            assert_eq!(pos.get_turn(), Some(0));
            assert!(!pos.is_legal("nonsense"));
            assert!(!pos.is_legal(""));
        }
    }
}
