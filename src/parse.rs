use crate::game::parse_game_name;
use crate::players::clock::ClockType;
use crate::sprt::Model;
use crate::tournament::{PlayerSettings, SPRTSettings, TournamentSettings};
use clap::{Arg, ArgAction, Command, parser::ValueSource};
use std::{ffi::OsString, fs::read_to_string};

#[derive(PartialEq, Debug)]
pub enum ParseError {
    // Malformed CLI syntax (unknown flag, missing/invalid value for a
    // typed option, etc.), as detected by clap.
    Clap(String),
    ValueParse(String),
    FileParse(String),
    SettingsParse(String),
    MissingParameter(String),
    UnknownParameter(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Clap(msg) => write!(f, "{}", msg.trim_end()),
            Self::ValueParse(msg) => write!(f, "{}", msg),
            Self::FileParse(path) => write!(f, "Failed to read the file \"{}\"", path),
            Self::SettingsParse(msg) => write!(f, "Invalid settings file {}", msg),
            Self::MissingParameter(what) => write!(f, "Missing a value for \"{}\"", what),
            Self::UnknownParameter(what) => write!(f, "Unknown parameter \"{}\"", what),
        }
    }
}

fn parse_f32(value: &str) -> Result<f32, ParseError> {
    value
        .parse::<f32>()
        .map_err(|_| ParseError::ValueParse(format!("Failed to parse f32: {}", value)))
}

fn parse_ms(value: &str) -> Result<u128, ParseError> {
    value
        .parse::<u128>()
        .map_err(|_| ParseError::ValueParse(format!("Failed to parse a time in ms: {}", value)))
}

// Kept absolute so a saved results file still points at the same file when
// it's used from another directory
pub(crate) fn absolute_path(path: &str) -> String {
    std::path::absolute(path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

// One position per line. Blank lines and lines starting with '#' are skipped,
// so opening files can carry a header or notes.
pub(crate) fn parse_fen_list(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

pub(crate) const CLOCK_KEYS: [&str; 3] = ["tc", "st", "depth"];

// Parses one of a player's time control keys: tc=<ms>[+<inc ms>] for a
// clock, st=<ms> for a fixed time per move, or depth=<n>.
pub(crate) fn parse_clock(key: &str, value: &str) -> Result<ClockType, ParseError> {
    match key {
        "tc" => {
            let (base, inc) = value.split_once('+').unwrap_or((value, "0"));
            Ok(ClockType::Time(parse_ms(base)?, parse_ms(inc)?))
        }
        "st" => Ok(ClockType::Movetime(parse_ms(value)?)),
        "depth" => Ok(ClockType::Depth(value.parse::<u8>().map_err(|_| {
            ParseError::ValueParse(format!("Failed to parse a depth: {}", value))
        })?)),
        _ => Err(ParseError::UnknownParameter(key.to_string())),
    }
}

// A player can only have one time control
pub(crate) fn set_clock(slot: &mut Option<ClockType>, clock: ClockType) -> Result<(), ParseError> {
    if slot.is_some() {
        return Err(ParseError::ValueParse(
            "A player can only have one of tc, st or depth".to_string(),
        ));
    }
    *slot = Some(clock);
    Ok(())
}

// Everything but --threads, --updates and --verbose
const RESULT_ARGS: [&str; 11] = [
    "settings",
    "game",
    "games",
    "fens",
    "results",
    "tc",
    "st",
    "depth",
    "player",
    "trinomial",
    "pentanomial",
];

fn cli() -> Command {
    Command::new("cutergames")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(Arg::new("settings").long("settings"))
        .arg(Arg::new("game").long("game"))
        .arg(
            Arg::new("games")
                .long("games")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("updates")
                .long("updates")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(Arg::new("fens").long("fens"))
        .arg(Arg::new("results").long("results"))
        .arg(Arg::new("resume").long("resume"))
        .arg(Arg::new("tc").long("tc"))
        .arg(Arg::new("st").long("st"))
        .arg(Arg::new("depth").long("depth"))
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("player")
                .long("player")
                .num_args(1..)
                .action(ArgAction::Append),
        )
        .arg(Arg::new("trinomial").long("trinomial").num_args(0..))
        .arg(Arg::new("pentanomial").long("pentanomial").num_args(0..))
}

// Parses a single `--player key=value ...` group into a PlayerSettings.
fn parse_player_tokens<'a>(
    tokens: impl Iterator<Item = &'a String>,
) -> Result<PlayerSettings, ParseError> {
    let mut player = PlayerSettings::default();
    let mut clock = None;

    for token in tokens {
        if token == "debug" {
            player.debug = true;
            continue;
        }

        match token.split_once('=') {
            Some((first, "")) => return Err(ParseError::MissingParameter(first.to_string())),
            Some(("name", second)) => player.name = second.to_string(),
            Some(("path", second)) => player.path = second.to_string(),
            Some(("proto", second)) => player.proto = second.to_string(),
            Some(("parameters", second)) => player.parameters = second.to_string(),
            Some((first, second)) if CLOCK_KEYS.contains(&first) => {
                set_clock(&mut clock, parse_clock(first, second)?)?
            }
            Some((first, second)) if first.starts_with("option.") => {
                let name = &first["option.".len()..];
                // The name and value go straight into a line sent to the
                // engine, so they can't contain line breaks
                if name.is_empty() || name.contains(['\n', '\r']) || second.contains(['\n', '\r']) {
                    return Err(ParseError::UnknownParameter(token.to_string()));
                }
                player.options.push((name.to_string(), second.to_string()));
            }
            Some((first, _)) => return Err(ParseError::UnknownParameter(first.to_string())),
            None => return Err(ParseError::UnknownParameter(token.to_string())),
        }
    }

    if player.path.is_empty() {
        return Err(ParseError::MissingParameter("path".to_string()));
    }

    player.clock = clock;

    Ok(player)
}

pub(crate) fn parse_sprt_model(value: &str) -> Result<Model, ParseError> {
    Model::from_name(value).ok_or_else(|| {
        ParseError::ValueParse(format!(
            "Unknown SPRT model \"{}\", expected normalized, logistic or bayesian",
            value
        ))
    })
}

// Parses a single `--trinomial`/`--pentanomial key=value ...` group into a SPRTSettings.
fn parse_sprt_tokens<'a>(
    tokens: impl Iterator<Item = &'a String>,
) -> Result<SPRTSettings, ParseError> {
    let mut sprt = SPRTSettings::default();

    for token in tokens {
        if token == "autostop" {
            sprt.autostop = true;
            continue;
        }

        match token.split_once('=') {
            Some((first, "")) => return Err(ParseError::MissingParameter(first.to_string())),
            Some(("alpha", second)) => sprt.alpha = parse_f32(second)?,
            Some(("beta", second)) => sprt.beta = parse_f32(second)?,
            Some(("elo0", second)) => sprt.elo0 = parse_f32(second)?,
            Some(("elo1", second)) => sprt.elo1 = parse_f32(second)?,
            Some(("model", second)) => sprt.model = parse_sprt_model(second)?,
            Some((first, _)) => return Err(ParseError::UnknownParameter(first.to_string())),
            None => return Err(ParseError::UnknownParameter(token.to_string())),
        }
    }

    Ok(sprt)
}

pub fn parse<T>(iter: impl Iterator<Item = T>) -> Result<TournamentSettings, ParseError>
where
    T: Into<OsString>,
{
    let matches = cli()
        .try_get_matches_from(
            std::iter::once(OsString::from("cutergames")).chain(iter.map(Into::into)),
        )
        .map_err(|err| ParseError::Clap(err.to_string()))?;

    // A resumed match is the saved one, so only options that can't change
    // its result can be given alongside
    if matches.get_one::<String>("resume").is_some()
        && let Some(arg) = RESULT_ARGS
            .iter()
            .find(|arg| matches.value_source(arg) == Some(ValueSource::CommandLine))
    {
        return Err(ParseError::ValueParse(format!(
            "--{} can't be used with --resume, which continues the saved match",
            arg
        )));
    }

    // The settings file is the base; anything else on the command line
    // overrides it
    let mut settings = match (
        matches.get_one::<String>("resume"),
        matches.get_one::<String>("settings"),
    ) {
        (Some(path), _) => {
            let mut settings = crate::settings_file::load(path)?;
            settings.results = Some(path.to_string());
            settings.resume = true;
            settings
        }
        (None, Some(path)) => crate::settings_file::load(path)?,
        (None, None) => TournamentSettings::default(),
    };

    if let Some(value) = matches.get_one::<String>("game") {
        settings.game = parse_game_name(value)
            .ok_or_else(|| ParseError::ValueParse(format!("Unknown game: {}", value)))?;
    }

    if let Some(&value) = matches.get_one::<usize>("games") {
        settings.num_games = Some(value);
    }

    if let Some(&value) = matches.get_one::<usize>("threads") {
        settings.num_threads = value;
    }

    if let Some(&value) = matches.get_one::<usize>("updates") {
        settings.update_frequency = value;
    }

    if let Some(path) = matches.get_one::<String>("fens") {
        settings.fens = parse_fen_list(
            &read_to_string(path).map_err(|_| ParseError::FileParse(path.to_string()))?,
        );
        settings.openings = Some(absolute_path(path));
    }

    if let Some(path) = matches.get_one::<String>("results") {
        settings.results = Some(path.to_string());
    }

    if matches.get_flag("verbose") {
        settings.verbose = true;
    }

    // Replaces the settings file's time control rather than conflicting
    // with it
    let mut clock = None;
    for key in CLOCK_KEYS {
        if let Some(value) = matches.get_one::<String>(key) {
            set_clock(&mut clock, parse_clock(key, value)?)?;
        }
    }
    if let Some(clock) = clock {
        settings.clock = clock;
    }

    // Players given on the command line replace the settings file's players
    // rather than adding to them
    if let Some(occurrences) = matches.get_occurrences::<String>("player") {
        settings.players.clear();
        for occurrence in occurrences {
            settings.players.push(parse_player_tokens(occurrence)?);
        }
    }

    if let Some(values) = matches.get_many::<String>("trinomial") {
        settings.sprt_trinomial = Some(parse_sprt_tokens(values)?);
    }

    if let Some(values) = matches.get_many::<String>("pentanomial") {
        settings.sprt_pentanomial = Some(parse_sprt_tokens(values)?);
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fen_list_skips_comments_and_blank_lines() {
        let text = "# header\nstartpos\n\n  # indented comment\nx5o/7/7/7/7/7/o5x x 0 1  \n";
        assert_eq!(
            parse_fen_list(text),
            vec![
                "startpos".to_string(),
                "x5o/7/7/7/7/7/o5x x 0 1".to_string()
            ]
        );
        assert!(parse_fen_list("# only a comment\n").is_empty());
    }

    #[test]
    fn success() {
        assert_eq!(
            parse(
                vec![
                    "--player",
                    "name=Some name 1",
                    "path=test1",
                    "parameters=Some thing long here",
                    "proto=test",
                    "--player",
                    "name=Some name 2",
                    "path=test2",
                    "parameters=--one --two two",
                    "proto=test",
                    "--games",
                    "123",
                    "--threads",
                    "4",
                    "--verbose",
                ]
                .into_iter()
            ),
            Ok(TournamentSettings {
                players: vec![
                    PlayerSettings {
                        name: "Some name 1".to_string(),
                        path: "test1".to_string(),
                        proto: "test".to_string(),
                        parameters: "Some thing long here".to_string(),
                        options: vec![],
                        debug: false,
                        clock: None,
                    },
                    PlayerSettings {
                        name: "Some name 2".to_string(),
                        path: "test2".to_string(),
                        proto: "test".to_string(),
                        parameters: "--one --two two".to_string(),
                        options: vec![],
                        debug: false,
                        clock: None,
                    }
                ],
                fens: vec![],
                openings: None,
                results: None,
                resume: false,
                game: crate::game::Game::Ataxx,
                num_threads: 4,
                num_games: Some(123),
                update_frequency: 10,
                sprt_trinomial: None,
                sprt_pentanomial: None,
                verbose: true,
                clock: ClockType::default(),
            })
        );
    }

    #[test]
    fn failure() {
        assert!(
            parse(
                vec![
                    "--player",
                    "name=Some name 1",
                    "--player",
                    "name=Some name 2",
                    "path=test2",
                    "--games",
                    "123",
                    "--threads",
                    "4",
                    "--verbose",
                ]
                .into_iter()
            )
            .is_err()
        );
    }

    #[test]
    fn sprt() {
        let settings = parse(
            vec![
                "--trinomial",
                "alpha=0.05",
                "beta=0.05",
                "elo0=0",
                "elo1=10",
                "autostop",
                "--pentanomial",
                "alpha=0.01",
                "beta=0.02",
                "elo0=1",
                "elo1=11",
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            settings.sprt_trinomial,
            Some(SPRTSettings {
                alpha: 0.05,
                beta: 0.05,
                elo0: 0.0,
                elo1: 10.0,
                autostop: true,
                model: Model::default(),
            })
        );
        assert_eq!(
            settings.sprt_pentanomial,
            Some(SPRTSettings {
                alpha: 0.01,
                beta: 0.02,
                elo0: 1.0,
                elo1: 11.0,
                autostop: false,
                model: Model::default(),
            })
        );
    }

    #[test]
    fn player_debug_flag() {
        let settings =
            parse(vec!["--player", "path=test1", "debug", "--player", "path=test2"].into_iter())
                .unwrap();

        assert!(settings.players[0].debug);
        assert!(!settings.players[1].debug);
    }

    #[test]
    fn player_options() {
        let settings = parse(
            vec![
                "--player",
                "path=test1",
                "option.Hash=128",
                "option.Skill Level=5",
                "--player",
                "path=test2",
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            settings.players[0].options,
            vec![
                ("Hash".to_string(), "128".to_string()),
                ("Skill Level".to_string(), "5".to_string())
            ]
        );
        assert!(settings.players[1].options.is_empty());
    }

    #[test]
    fn player_options_rejects_bad_input() {
        for token in [
            "option.=1",
            "option.Hash=",
            "option.Hash=1\n2",
            "option.Ha\nsh=1",
        ] {
            assert!(
                parse(vec!["--player", "path=test1", token].into_iter()).is_err(),
                "{token:?} should be rejected"
            );
        }
    }

    // Writes `json` to a file only this test uses and returns its path
    fn settings_file(name: &str, json: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "cutergames_test_{}_{}.json",
            std::process::id(),
            name
        ));
        std::fs::write(&path, json).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn settings_file_is_the_base() {
        let path = settings_file(
            "base",
            r#"{"players": [{"path": "a"}, {"path": "b"}], "games": 10, "threads": 2}"#,
        );
        let settings = parse(vec!["--settings", path.as_str()].into_iter()).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(settings.players.len(), 2);
        assert_eq!(settings.num_games, Some(10));
        assert_eq!(settings.num_threads, 2);
    }

    #[test]
    fn command_line_overrides_settings_file() {
        let path = settings_file(
            "overrides",
            r#"{
                "players": [{"path": "a"}, {"path": "b"}],
                "games": 10,
                "threads": 2,
                "verbose": true,
                "trinomial": {"alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 5}
            }"#,
        );
        let settings = parse(
            vec![
                "--settings",
                path.as_str(),
                "--games",
                "99",
                "--player",
                "path=c",
                "--trinomial",
                "alpha=0.1",
                "beta=0.1",
                "elo0=1",
                "elo1=2",
            ]
            .into_iter(),
        )
        .unwrap();
        std::fs::remove_file(&path).unwrap();

        // Overridden
        assert_eq!(settings.num_games, Some(99));
        assert_eq!(settings.players.len(), 1);
        assert_eq!(settings.players[0].path, "c");
        assert_eq!(settings.sprt_trinomial.unwrap().alpha, 0.1);
        // Not mentioned on the command line, so kept from the file
        assert_eq!(settings.num_threads, 2);
        assert!(settings.verbose);
    }

    #[test]
    fn missing_or_bad_settings_file() {
        assert!(parse(vec!["--settings", "/definitely/not/a/file.json"].into_iter()).is_err());

        let path = settings_file("bad", r#"{"pgn": {}}"#);
        let result = parse(vec!["--settings", path.as_str()].into_iter());
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(ParseError::SettingsParse(_))));
    }

    #[test]
    fn sprt_model() {
        let settings = parse(
            vec![
                "--trinomial",
                "alpha=0.05",
                "beta=0.05",
                "elo0=0",
                "elo1=2",
                "model=bayesian",
                "--pentanomial",
                "alpha=0.05",
                "beta=0.05",
                "elo0=0",
                "elo1=2",
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(settings.sprt_trinomial.unwrap().model, Model::Bayesian);
        // Defaults to normalized
        assert_eq!(settings.sprt_pentanomial.unwrap().model, Model::Normalized);

        assert!(parse(vec!["--trinomial", "model=nonsense"].into_iter()).is_err());
    }

    #[test]
    fn player_tc() {
        let settings = parse(
            vec![
                "--player",
                "path=test1",
                "st=100",
                "--player",
                "path=test2",
                "depth=6",
                "--player",
                "path=test3",
                "tc=60000+600",
                "--player",
                "path=test4",
                "tc=1000+10",
                "--player",
                "path=test5",
                "tc=10000",
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(settings.players[0].clock, Some(ClockType::Movetime(100)));
        assert_eq!(settings.players[1].clock, Some(ClockType::Depth(6)));
        assert_eq!(settings.players[2].clock, Some(ClockType::Time(60000, 600)));
        assert_eq!(settings.players[3].clock, Some(ClockType::Time(1000, 10)));
        assert_eq!(settings.players[4].clock, Some(ClockType::Time(10000, 0)));

        // Unset, uses the tournament's clock, which defaults to st=10
        let settings = parse(vec!["--player", "path=test1"].into_iter()).unwrap();
        assert_eq!(settings.players[0].clock, None);
        assert_eq!(
            settings.player_clock(&settings.players[0]),
            ClockType::Movetime(10)
        );

        for token in [
            "tc=nonsense",
            "tc=1+",
            "tc=+1",
            "tc=1+2+3",
            "tc=-1",
            "tc=0.5",
            "tc=1000+0.5",
            "st=nan",
            "depth=1.5",
            "depth=-1",
        ] {
            assert!(
                matches!(
                    parse(vec!["--player", "path=test1", token].into_iter()),
                    Err(ParseError::ValueParse(_))
                ),
                "{token:?} should be rejected"
            );
        }

        // Only one time control per player
        assert!(matches!(
            parse(vec!["--player", "path=test1", "tc=1000+10", "depth=6"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
        assert!(matches!(
            parse(vec!["--player", "path=test1", "st=100", "st=200"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
    }

    #[test]
    fn global_tc() {
        let settings = parse(
            vec![
                "--tc",
                "1000+10",
                "--player",
                "path=test1",
                "--player",
                "path=test2",
                "tc=2000+20",
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(settings.clock, ClockType::Time(1000, 10));
        assert_eq!(
            settings.player_clock(&settings.players[0]),
            ClockType::Time(1000, 10)
        );
        assert_eq!(
            settings.player_clock(&settings.players[1]),
            ClockType::Time(2000, 20)
        );

        let settings = parse(vec!["--depth", "6"].into_iter()).unwrap();
        assert_eq!(settings.clock, ClockType::Depth(6));

        assert!(matches!(
            parse(vec!["--tc", "1000+10", "--st", "50"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
        assert!(matches!(
            parse(vec!["--st", "0.5"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
    }

    #[test]
    fn resume_only_takes_safe_options() {
        for args in [
            vec!["--resume", "x.json", "--games", "10"],
            vec!["--resume", "x.json", "--tc", "1000+10"],
            vec!["--resume", "x.json", "--settings", "y.json"],
            vec!["--resume", "x.json", "--player", "path=a"],
            vec!["--resume", "x.json", "--pentanomial", "autostop"],
            vec!["--resume", "x.json", "--results", "y.json"],
        ] {
            assert!(
                matches!(
                    parse(args.clone().into_iter()),
                    Err(ParseError::ValueParse(_))
                ),
                "{args:?} should be rejected"
            );
        }

        // Allowed options get as far as reading the file
        assert!(matches!(
            parse(
                vec![
                    "--resume",
                    "/definitely/not/a/file.json",
                    "--threads",
                    "4",
                    "--verbose"
                ]
                .into_iter()
            ),
            Err(ParseError::FileParse(_))
        ));
    }

    #[test]
    fn values_are_consumed() {
        // "10" is the value of --updates, not a parameter of its own
        let settings = parse(vec!["--updates", "10", "--threads", "2"].into_iter()).unwrap();
        assert_eq!(settings.update_frequency, 10);
        assert_eq!(settings.num_threads, 2);
    }

    #[test]
    fn game() {
        let settings = parse(vec!["--game", "chess"].into_iter()).unwrap();
        assert_eq!(settings.game, crate::game::Game::Chess);

        assert!(matches!(
            parse(vec!["--game", "nonsense"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
    }

    #[test]
    fn bad_values() {
        // Malformed numbers are reported by clap rather than panicking
        assert!(matches!(
            parse(vec!["--games", "many"].into_iter()),
            Err(ParseError::Clap(_))
        ));
        // Malformed sub-values inside a group are still our own errors
        assert!(matches!(
            parse(vec!["--trinomial", "alpha=loads"].into_iter()),
            Err(ParseError::ValueParse(_))
        ));
        // Missing values
        assert!(matches!(
            parse(vec!["--threads"].into_iter()),
            Err(ParseError::Clap(_))
        ));
        // Unknown flags
        assert!(matches!(
            parse(vec!["--treads", "4"].into_iter()),
            Err(ParseError::Clap(_))
        ));
    }
}
