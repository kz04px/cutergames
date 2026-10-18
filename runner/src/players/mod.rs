use crate::game::Game;
use crate::players::{
    clock::ClockType, tei::TEIEngine, uai::UAIEngine, uci::UCIEngine, ugi::UGIEngine,
};

pub mod clock;
mod process;
pub use process::quit_all_engines;
pub mod tei;
pub mod uai;
pub mod uci;
pub mod ugi;

#[derive(PartialEq, Debug)]
pub enum Protocol {
    UGI,
    UAI,
    UCI,
    TEI,
}

#[must_use]
pub fn create(
    name: &str,
    path: &str,
    parameters: &str,
    proto: Protocol,
    game: Game,
    clock: ClockType,
    debug: bool,
) -> Box<dyn Player> {
    match proto {
        Protocol::UGI => Box::new(UGIEngine::new(name, path, parameters, clock, debug)),
        Protocol::UAI => Box::new(UAIEngine::new(name, path, parameters, game, clock, debug)),
        Protocol::UCI => Box::new(UCIEngine::new(name, path, parameters, game, clock, debug)),
        Protocol::TEI => Box::new(TEIEngine::new(name, path, parameters, game, clock, debug)),
    }
}

#[must_use]
pub fn get_protocol(name: &str) -> Option<Protocol> {
    match name.to_lowercase().as_str() {
        "ugi" => Some(Protocol::UGI),
        "uai" => Some(Protocol::UAI),
        "uci" => Some(Protocol::UCI),
        "tei" => Some(Protocol::TEI),
        _ => None,
    }
}

pub trait Player {
    fn init(&mut self);

    // Sends `setoption`; meant to be called after init() and before isready()
    fn set_option(&mut self, name: &str, value: &str);

    fn isready(&mut self);

    fn set_position(&mut self, fen: &str);

    fn get_move(&mut self, p1_clock: ClockType, p2_clock: ClockType) -> Option<String>;

    fn makemove(&mut self, mvstr: &str) -> bool;

    #[must_use]
    fn get_clock(&mut self) -> &mut ClockType;

    #[must_use]
    fn is_gameover(&mut self) -> bool;

    #[must_use]
    fn is_legal(&mut self, mvstr: &str) -> bool;

    #[must_use]
    fn get_turn(&mut self) -> Option<usize>;

    #[must_use]
    fn query_result(&mut self) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        assert_eq!(get_protocol("ugi"), Some(Protocol::UGI));
        assert_eq!(get_protocol("Ugi"), Some(Protocol::UGI));
        assert_eq!(get_protocol("UGI"), Some(Protocol::UGI));

        assert_eq!(get_protocol("uai"), Some(Protocol::UAI));
        assert_eq!(get_protocol("Uai"), Some(Protocol::UAI));
        assert_eq!(get_protocol("UAI"), Some(Protocol::UAI));

        assert_eq!(get_protocol("uci"), Some(Protocol::UCI));
        assert_eq!(get_protocol("Uci"), Some(Protocol::UCI));
        assert_eq!(get_protocol("UCI"), Some(Protocol::UCI));

        assert_eq!(get_protocol("tei"), Some(Protocol::TEI));
        assert_eq!(get_protocol("Tei"), Some(Protocol::TEI));
        assert_eq!(get_protocol("TEI"), Some(Protocol::TEI));
    }

    #[test]
    fn failure() {
        assert_eq!(get_protocol(""), None);
        assert_eq!(get_protocol("ugii"), None);
        assert_eq!(get_protocol("test"), None);
    }
}
