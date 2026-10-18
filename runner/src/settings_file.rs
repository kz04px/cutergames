use crate::game::parse_game_name;
use crate::parse::{
    CLOCK_KEYS, ParseError, absolute_path, parse_clock, parse_fen_list, parse_sprt_model, set_clock,
};
use crate::players::clock::ClockType;
use crate::sprt::Model;
use crate::tournament::{PlayerSettings, SPRTSettings, TournamentSettings};
use serde::{Deserialize, de::IgnoredAny};
use serde_json::{Map, Value};
use std::fs::read_to_string;

// The JSON form of the settings. Every field is optional, as on the command
// line, and unknown keys are an error so that a typo (or a key that isn't
// supported) doesn't silently do nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSettings {
    players: Option<Vec<FilePlayer>>,
    game: Option<String>,
    openings: Option<String>,
    games: Option<usize>,
    threads: Option<usize>,
    updates: Option<usize>,
    trinomial: Option<FileSprt>,
    pentanomial: Option<FileSprt>,
    verbose: Option<bool>,
    // The time control for players that don't set their own
    tc: Option<Value>,
    st: Option<Value>,
    depth: Option<Value>,
    // What a results file saved, which says nothing about how to run the
    // match, so a results file can be loaded as a settings file
    #[allow(dead_code)]
    results: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePlayer {
    name: Option<String>,
    path: String,
    proto: Option<String>,
    parameters: Option<String>,
    // Time controls, written as on the command line, as a string or number
    tc: Option<Value>,
    st: Option<Value>,
    depth: Option<Value>,
    debug: Option<bool>,
    // Sent with `setoption`, in the order written
    options: Option<Map<String, Value>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSprt {
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(default)]
    autostop: bool,
    model: Option<String>,
    alpha: f32,
    beta: f32,
    elo0: f32,
    elo1: f32,
}

fn enabled_by_default() -> bool {
    true
}

fn option_value(name: &str, value: &Value) -> Result<String, ParseError> {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => {
            return Err(ParseError::ValueParse(format!(
                "Option \"{}\" must be a string, number or boolean",
                name
            )));
        }
    };

    // Name and value are sent as a single line to the engine
    if name.is_empty()
        || name.contains(['\n', '\r'])
        || text.is_empty()
        || text.contains(['\n', '\r'])
    {
        return Err(ParseError::ValueParse(format!(
            "Invalid engine option \"{}\"",
            name
        )));
    }

    Ok(text)
}

fn clock_value(key: &str, value: &Value) -> Result<String, ParseError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(ParseError::ValueParse(format!(
            "\"{}\" must be a string or number",
            key
        ))),
    }
}

// The tc, st and depth keys, of which at most one can be given
fn convert_clock(values: [&Option<Value>; 3]) -> Result<Option<ClockType>, ParseError> {
    let mut clock = None;
    for (key, value) in CLOCK_KEYS.iter().zip(values) {
        if let Some(value) = value {
            set_clock(&mut clock, parse_clock(key, &clock_value(key, value)?)?)?;
        }
    }
    Ok(clock)
}

fn convert_player(player: FilePlayer) -> Result<PlayerSettings, ParseError> {
    let mut out = PlayerSettings::default();

    if let Some(name) = player.name {
        out.name = name;
    }
    out.path = player.path;
    if let Some(proto) = player.proto {
        out.proto = proto;
    }
    if let Some(parameters) = player.parameters {
        out.parameters = parameters;
    }
    out.clock = convert_clock([&player.tc, &player.st, &player.depth])?;
    if let Some(debug) = player.debug {
        out.debug = debug;
    }
    if let Some(options) = player.options {
        for (name, value) in &options {
            out.options.push((name.clone(), option_value(name, value)?));
        }
    }

    if out.path.is_empty() {
        return Err(ParseError::MissingParameter("path".to_string()));
    }

    Ok(out)
}

fn convert_sprt(sprt: Option<FileSprt>) -> Result<Option<SPRTSettings>, ParseError> {
    let Some(sprt) = sprt.filter(|s| s.enabled) else {
        return Ok(None);
    };

    Ok(Some(SPRTSettings {
        alpha: sprt.alpha,
        beta: sprt.beta,
        elo0: sprt.elo0,
        elo1: sprt.elo1,
        autostop: sprt.autostop,
        model: match sprt.model {
            Some(name) => parse_sprt_model(&name)?,
            None => Model::default(),
        },
    }))
}

fn from_json(text: &str) -> Result<TournamentSettings, ParseError> {
    let file: FileSettings =
        serde_json::from_str(text).map_err(|err| ParseError::SettingsParse(err.to_string()))?;

    let mut settings = TournamentSettings::default();

    if let Some(clock) = convert_clock([&file.tc, &file.st, &file.depth])? {
        settings.clock = clock;
    }
    if let Some(players) = file.players {
        for player in players {
            settings.players.push(convert_player(player)?);
        }
    }
    if let Some(game) = file.game {
        settings.game = parse_game_name(&game)
            .ok_or_else(|| ParseError::ValueParse(format!("Unknown game: {}", game)))?;
    }
    if let Some(path) = file.openings {
        settings.fens = parse_fen_list(
            &read_to_string(&path).map_err(|_| ParseError::FileParse(path.clone()))?,
        );
        settings.openings = Some(absolute_path(&path));
    }
    if let Some(games) = file.games {
        settings.num_games = Some(games);
    }
    if let Some(threads) = file.threads {
        settings.num_threads = threads;
    }
    if let Some(updates) = file.updates {
        settings.update_frequency = updates;
    }
    settings.sprt_trinomial = convert_sprt(file.trinomial)?;
    settings.sprt_pentanomial = convert_sprt(file.pentanomial)?;
    if let Some(verbose) = file.verbose {
        settings.verbose = verbose;
    }

    Ok(settings)
}

// JSON doesn't have f32, and widening one to f64 shows its rounding error
// (0.05 becomes 0.05000000074505806), so go through its shortest decimal form
pub(crate) fn f32_value(value: f32) -> Value {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

fn sprt_json(sprt: &SPRTSettings) -> Value {
    let mut out = Map::new();
    out.insert("enabled".to_string(), true.into());
    out.insert("autostop".to_string(), sprt.autostop.into());
    out.insert("model".to_string(), sprt.model.name().into());
    out.insert("alpha".to_string(), f32_value(sprt.alpha));
    out.insert("beta".to_string(), f32_value(sprt.beta));
    out.insert("elo0".to_string(), f32_value(sprt.elo0));
    out.insert("elo1".to_string(), f32_value(sprt.elo1));
    Value::Object(out)
}

fn player_json(player: &PlayerSettings) -> Value {
    let mut out = Map::new();
    out.insert("name".to_string(), player.name.clone().into());
    out.insert("path".to_string(), player.path.clone().into());
    out.insert("proto".to_string(), player.proto.clone().into());
    out.insert("parameters".to_string(), player.parameters.clone().into());
    if let Some(clock) = player.clock {
        let (key, value) = clock.key_value();
        out.insert(key.to_string(), value.into());
    }
    out.insert("debug".to_string(), player.debug.into());
    if !player.options.is_empty() {
        let options = player
            .options
            .iter()
            .map(|(name, value)| (name.clone(), Value::String(value.clone())))
            .collect();
        out.insert("options".to_string(), Value::Object(options));
    }
    Value::Object(out)
}

// The settings as a settings file, with every value written out rather than
// left to a default, so loading it gives the same settings back
pub fn to_json(settings: &TournamentSettings) -> Value {
    let mut out = Map::new();
    out.insert("game".to_string(), settings.game.name().into());
    if let Some(path) = &settings.openings {
        out.insert("openings".to_string(), path.clone().into());
    }
    if let Some(games) = settings.num_games {
        out.insert("games".to_string(), games.into());
    }
    out.insert("threads".to_string(), settings.num_threads.into());
    out.insert("updates".to_string(), settings.update_frequency.into());
    let (key, value) = settings.clock.key_value();
    out.insert(key.to_string(), value.into());
    for (name, sprt) in [
        ("trinomial", &settings.sprt_trinomial),
        ("pentanomial", &settings.sprt_pentanomial),
    ] {
        if let Some(sprt) = sprt {
            out.insert(name.to_string(), sprt_json(sprt));
        }
    }
    out.insert("verbose".to_string(), settings.verbose.into());
    out.insert(
        "players".to_string(),
        Value::Array(settings.players.iter().map(player_json).collect()),
    );
    Value::Object(out)
}

pub fn load(path: &str) -> Result<TournamentSettings, ParseError> {
    let text = read_to_string(path).map_err(|_| ParseError::FileParse(path.to_string()))?;
    from_json(&text).map_err(|err| match err {
        ParseError::SettingsParse(msg) => ParseError::SettingsParse(format!("{}: {}", path, msg)),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_settings() {
        let settings = from_json(
            r#"{
                "players": [
                    {"name": "A", "path": "a", "proto": "ugi", "depth": 5,
                     "options": {"Hash": "128", "Threads": 2, "Ponder": false}},
                    {"path": "b", "debug": true, "parameters": "--x --y"}
                ],
                "game": "chess",
                "games": 50,
                "threads": 3,
                "updates": 7,
                "trinomial": {"autostop": true, "alpha": 0.05, "beta": 0.1, "elo0": 0, "elo1": 5},
                "verbose": true
            }"#,
        )
        .unwrap();

        assert_eq!(settings.players.len(), 2);
        assert_eq!(settings.players[0].name, "A");
        assert_eq!(settings.players[0].clock, Some(ClockType::Depth(5)));
        assert_eq!(
            settings.players[0].options,
            vec![
                ("Hash".to_string(), "128".to_string()),
                ("Threads".to_string(), "2".to_string()),
                ("Ponder".to_string(), "false".to_string()),
            ]
        );
        assert!(settings.players[1].debug);
        assert_eq!(settings.players[1].parameters, "--x --y");
        assert_eq!(settings.num_games, Some(50));
        assert_eq!(settings.num_threads, 3);
        assert_eq!(settings.update_frequency, 7);
        assert!(settings.verbose);
        let tri = settings.sprt_trinomial.unwrap();
        assert!(tri.autostop);
        assert_eq!(tri.beta, 0.1);
        assert!(settings.sprt_pentanomial.is_none());
    }

    #[test]
    fn player_clocks() {
        let settings = from_json(
            r#"{
                "players": [
                    {"path": "a", "tc": "1000+10"},
                    {"path": "b", "tc": 10000},
                    {"path": "c", "st": 50},
                    {"path": "d", "depth": "6"}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(settings.players[0].clock, Some(ClockType::Time(1000, 10)));
        assert_eq!(settings.players[1].clock, Some(ClockType::Time(10000, 0)));
        assert_eq!(settings.players[2].clock, Some(ClockType::Movetime(50)));
        assert_eq!(settings.players[3].clock, Some(ClockType::Depth(6)));
    }

    #[test]
    fn global_clock() {
        let settings = from_json(
            r#"{
                "tc": "1000+10",
                "players": [
                    {"path": "a"},
                    {"path": "b", "tc": "2000+20"}
                ]
            }"#,
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
    }

    #[test]
    fn example_settings() {
        // The example's openings path is a placeholder, so it can't go
        // through from_json
        let file: FileSettings = serde_json::from_str(include_str!("../../example.json")).unwrap();
        assert_eq!(
            convert_clock([&file.tc, &file.st, &file.depth]).unwrap(),
            Some(ClockType::Time(1000, 10))
        );
        let players: Vec<_> = file
            .players
            .unwrap()
            .into_iter()
            .map(|p| convert_player(p).unwrap())
            .collect();
        assert_eq!(players[0].clock, None);
        assert_eq!(players[1].clock, Some(ClockType::Time(5000, 50)));
    }

    #[test]
    fn to_json_round_trips() {
        let dir = std::env::temp_dir().join(format!("cutergames-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let openings = dir.join("openings.txt");
        std::fs::write(&openings, "# header\nstartpos\nsomething else\n").unwrap();

        let settings = from_json(&format!(
            r#"{{
                "game": "tak",
                "openings": "{}",
                "games": 500,
                "threads": 8,
                "updates": 50,
                "tc": "1000+10",
                "pentanomial": {{"autostop": true, "alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 10}},
                "players": [
                    {{"name": "A", "path": "a", "proto": "ugi", "parameters": "--game tak"}},
                    {{"name": "B", "path": "b", "tc": "2000+20", "debug": true,
                      "options": {{"Hash": 128, "Ponder": false}}}}
                ]
            }}"#,
            openings.display()
        ))
        .unwrap();

        let json = to_json(&settings);
        assert_eq!(json["pentanomial"]["alpha"], serde_json::json!(0.05));
        assert_eq!(json["tc"], "1000+10");
        assert_eq!(json["players"][0].get("tc"), None);
        assert_eq!(json["players"][1]["tc"], "2000+20");
        assert_eq!(from_json(&json.to_string()).unwrap(), settings);

        // A results file loads as the settings it saved
        let mut json = json;
        json["results"] = serde_json::json!({"games": 10, "anything": [1, 2, 3]});
        assert_eq!(from_json(&json.to_string()).unwrap(), settings);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn disabled_sprt_is_none() {
        let settings = from_json(
            r#"{"pentanomial": {"enabled": false, "alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 5}}"#,
        )
        .unwrap();
        assert!(settings.sprt_pentanomial.is_none());
    }

    #[test]
    fn sprt_model() {
        let settings = from_json(
            r#"{
                "trinomial": {"alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 2, "model": "logistic"},
                "pentanomial": {"alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 2}
            }"#,
        )
        .unwrap();
        assert_eq!(settings.sprt_trinomial.unwrap().model, Model::Logistic);
        assert_eq!(settings.sprt_pentanomial.unwrap().model, Model::Normalized);

        assert!(
            from_json(
                r#"{"trinomial": {"alpha": 0.05, "beta": 0.05, "elo0": 0, "elo1": 2, "model": "x"}}"#
            )
            .is_err()
        );
    }

    #[test]
    fn empty_object_is_the_defaults() {
        assert_eq!(from_json("{}").unwrap(), TournamentSettings::default());
    }

    #[test]
    fn rejects_bad_input() {
        for text in [
            r#"{"pgn": {}}"#,
            r#"{"players": [{"name": "no path"}]}"#,
            r#"{"players": [{"path": "a", "tc": "bogus"}]}"#,
            r#"{"players": [{"path": "a", "tc": "1000+10", "st": 100}]}"#,
            r#"{"players": [{"path": "a", "depth": true}]}"#,
            r#"{"tc": "1000+10", "depth": 6}"#,
            r#"{"players": [{"path": "a", "st": 0.5}]}"#,
            r#"{"players": [{"path": "a", "options": {"Hash": ""}}]}"#,
            r#"{"players": [{"path": "a", "options": {"Hash": [1]}}]}"#,
            r#"{"game": "nonsense"}"#,
            r#"{"trinomial": {"alpha": 0.05}}"#,
            r#"{"openings": "/definitely/not/a/file"}"#,
            "not json",
        ] {
            assert!(from_json(text).is_err(), "{text:?} should be rejected");
        }
    }
}
