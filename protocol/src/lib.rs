use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// Games a client is expected to know how to play. Kept as a plain string
// (rather than depending on the `games` crate here) since the server never
// looks inside a game beyond checking the name -- only a connected client
// actually runs one.
pub const KNOWN_GAMES: &[&str] = &[
    "ataxx",
    "chess",
    "connect4",
    "droptaxx",
    "gomoku",
    "isolation",
    "pijersi",
    "tak",
];

// Mirrors the runner's `--player` keys, so a client can pass this straight
// through when it invokes the runner for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSpec {
    pub name: String,
    pub path: String,
    pub proto: String,
    #[serde(default)]
    pub parameters: String,
    // The runner's time control key for this player, passed through as it
    // is: e.g. "tc=1000+10", "st=100" or "depth=6".
    #[serde(default = "default_tc")]
    pub tc: String,
    // Engine options, sent with `setoption` in the order listed, e.g.
    // {"Hash": "128"}. Values may be strings, numbers or booleans.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub options: Map<String, Value>,
}

impl EngineSpec {
    // The options as (name, value) text, or why they can't be sent. The
    // runner takes them as `option.<name>=<value>` command line arguments, so
    // a name can't be empty or contain '=', and neither side can contain a
    // line break (each becomes one line sent to the engine).
    pub fn option_pairs(&self) -> Result<Vec<(String, String)>, String> {
        let mut pairs = Vec::with_capacity(self.options.len());

        for (name, value) in &self.options {
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) => number.to_string(),
                Value::Bool(flag) => flag.to_string(),
                _ => {
                    return Err(format!(
                        "Option \"{name}\" of engine \"{}\" must be a string, number or boolean",
                        self.name
                    ));
                }
            };

            if name.is_empty()
                || name.contains(['=', '\n', '\r'])
                || text.is_empty()
                || text.contains(['\n', '\r'])
            {
                return Err(format!(
                    "Invalid option \"{name}\" for engine \"{}\"",
                    self.name
                ));
            }

            pairs.push((name.clone(), text));
        }

        Ok(pairs)
    }

    // Only the key is checked here; the runner checks the value.
    pub fn check_tc(&self) -> Result<(), String> {
        match self.tc.split_once('=') {
            Some(("tc" | "st" | "depth", value))
                if !value.is_empty() && !value.contains(char::is_whitespace) =>
            {
                Ok(())
            }
            _ => Err(format!(
                "Invalid time control \"{}\" for engine \"{}\", expected tc=<ms>[+<inc ms>], st=<ms> or depth=<n>",
                self.tc, self.name
            )),
        }
    }
}

fn default_tc() -> String {
    "st=10".to_string()
}

// A unit of work submitted to the server: a match between two engines,
// played as `num_games` individual games, alternating colour each game the
// way the runner's own generator does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub game: String,
    pub player1: EngineSpec,
    pub player2: EngineSpec,
    pub fens: Vec<String>,
    pub num_games: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    #[serde(flatten)]
    pub spec: JobSpec,
}

// Checked both for a job loaded from a file at startup and one submitted
// to a running server, so the two can't drift.
pub fn validate_job(spec: &JobSpec) -> Result<(), String> {
    if !KNOWN_GAMES.contains(&spec.game.as_str()) {
        return Err(format!(
            "Unknown game \"{}\", expected one of {}",
            spec.game,
            KNOWN_GAMES.join(", ")
        ));
    }
    for player in [&spec.player1, &spec.player2] {
        player.check_tc()?;
        player.option_pairs()?;
    }
    if spec.fens.is_empty() {
        return Err("A job's \"fens\" list must not be empty".to_string());
    }
    if spec.num_games == 0 {
        return Err("A job's \"num_games\" must be at least 1".to_string());
    }

    Ok(())
}

// One game within a job, as handed to a client. `game_id` is the game's
// index within the job (0..num_games); which engine plays player1 alternates
// with its parity, and the opening is picked the same way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub job_id: u64,
    pub game_id: usize,
    pub game: String,
    pub player1: EngineSpec,
    pub player2: EngineSpec,
    pub fen: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GameOutcome {
    P1win,
    P2win,
    Draw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: u64,
    pub game_id: usize,
    pub outcome: Option<GameOutcome>,
    pub ply: usize,
}

// Sent client -> server, one JSON value per line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello { name: String },
    RequestTask,
    TaskResult(TaskResult),
    SubmitJob(Box<JobSpec>),
}

// Sent server -> client, one JSON value per line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome,
    Task(Box<Task>),
    // No work available right now; ask again later.
    NoWork,
    Shutdown,
    // The id a SubmitJob was assigned.
    JobSubmitted(u64),
    // A SubmitJob was rejected, e.g. it failed validate_job().
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_message_roundtrip() {
        let messages = [
            ClientMessage::Hello {
                name: "worker-1".to_string(),
            },
            ClientMessage::RequestTask,
            ClientMessage::TaskResult(TaskResult {
                job_id: 3,
                game_id: 7,
                outcome: Some(GameOutcome::Draw),
                ply: 42,
            }),
            ClientMessage::SubmitJob(Box::new(JobSpec {
                game: "ataxx".to_string(),
                player1: EngineSpec {
                    name: "A".to_string(),
                    path: "./a".to_string(),
                    proto: "ugi".to_string(),
                    parameters: String::new(),
                    tc: "st=10".to_string(),
                    options: Map::new(),
                },
                player2: EngineSpec {
                    name: "B".to_string(),
                    path: "./b".to_string(),
                    proto: "ugi".to_string(),
                    parameters: String::new(),
                    tc: "st=10".to_string(),
                    options: Map::new(),
                },
                fens: vec!["startpos".to_string()],
                num_games: 10,
            })),
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let back: ClientMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(
                serde_json::to_string(&back).unwrap(),
                serde_json::to_string(&msg).unwrap()
            );
        }
    }

    #[test]
    fn server_message_roundtrip() {
        let task = Task {
            job_id: 1,
            game_id: 0,
            game: "ataxx".to_string(),
            player1: EngineSpec {
                name: "A".to_string(),
                path: "./a".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=100".to_string(),
                options: Map::new(),
            },
            player2: EngineSpec {
                name: "B".to_string(),
                path: "./b".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=100".to_string(),
                options: Map::new(),
            },
            fen: "startpos".to_string(),
        };

        let messages = [
            ServerMessage::Welcome,
            ServerMessage::NoWork,
            ServerMessage::Shutdown,
            ServerMessage::JobSubmitted(5),
            ServerMessage::Error("bad job".to_string()),
        ];
        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let _: ServerMessage = serde_json::from_str(&json).unwrap();
        }

        let json = serde_json::to_string(&ServerMessage::Task(Box::new(task.clone()))).unwrap();
        let ServerMessage::Task(back) = serde_json::from_str(&json).unwrap() else {
            panic!("expected Task");
        };
        assert_eq!(back.job_id, task.job_id);
        assert_eq!(back.fen, task.fen);
    }

    #[test]
    fn engine_spec_tc_defaults() {
        let json = r#"{"name":"A","path":"./a","proto":"ugi"}"#;
        let spec: EngineSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.tc, "st=10");
        assert_eq!(spec.parameters, "");
    }

    fn valid_spec() -> JobSpec {
        JobSpec {
            game: "ataxx".to_string(),
            player1: EngineSpec {
                name: "A".to_string(),
                path: "./a".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=10".to_string(),
                options: Map::new(),
            },
            player2: EngineSpec {
                name: "B".to_string(),
                path: "./b".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=10".to_string(),
                options: Map::new(),
            },
            fens: vec!["startpos".to_string()],
            num_games: 1,
        }
    }

    #[test]
    fn validate_job_accepts_a_sane_spec() {
        assert!(validate_job(&valid_spec()).is_ok());
    }

    #[test]
    fn validate_job_accepts_every_game_the_runner_plays() {
        for game in [
            "ataxx",
            "chess",
            "connect4",
            "droptaxx",
            "gomoku",
            "isolation",
            "pijersi",
            "tak",
        ] {
            let mut spec = valid_spec();
            spec.game = game.to_string();
            assert!(validate_job(&spec).is_ok(), "{game}");
        }
    }

    #[test]
    fn validate_job_checks_tc() {
        for tc in ["tc=1000+10", "tc=10000", "st=100", "depth=6"] {
            let mut spec = valid_spec();
            spec.player2.tc = tc.to_string();
            assert!(validate_job(&spec).is_ok(), "{tc:?} should be accepted");
        }
        for tc in ["movetime:10", "1000+10", "tc=", "nodes=100", "st=1 0", ""] {
            let mut spec = valid_spec();
            spec.player2.tc = tc.to_string();
            assert!(validate_job(&spec).is_err(), "{tc:?} should be rejected");
        }
    }

    #[test]
    fn validate_job_rejects_unknown_game() {
        let mut spec = valid_spec();
        spec.game = "nonsense".to_string();
        assert!(validate_job(&spec).is_err());
    }

    #[test]
    fn validate_job_rejects_empty_fens() {
        let mut spec = valid_spec();
        spec.fens = vec![];
        assert!(validate_job(&spec).is_err());
    }

    #[test]
    fn validate_job_rejects_zero_games() {
        let mut spec = valid_spec();
        spec.num_games = 0;
        assert!(validate_job(&spec).is_err());
    }

    fn spec_with_options(options: &str) -> EngineSpec {
        let json = format!(r#"{{"name":"A","path":"./a","proto":"ugi","options":{options}}}"#);
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn engine_spec_options_default_to_none() {
        let json = r#"{"name":"A","path":"./a","proto":"ugi"}"#;
        let spec: EngineSpec = serde_json::from_str(json).unwrap();
        assert!(spec.options.is_empty());
        assert_eq!(spec.option_pairs(), Ok(vec![]));
        // And none are written back out
        assert!(!serde_json::to_string(&spec).unwrap().contains("options"));
    }

    #[test]
    fn options_keep_their_order_and_accept_scalars() {
        let spec = spec_with_options(r#"{"Hash":"128","Threads":2,"Ponder":false,"Another":"x"}"#);
        assert_eq!(
            spec.option_pairs().unwrap(),
            vec![
                ("Hash".to_string(), "128".to_string()),
                ("Threads".to_string(), "2".to_string()),
                ("Ponder".to_string(), "false".to_string()),
                ("Another".to_string(), "x".to_string()),
            ]
        );

        // The order survives a round trip over the wire
        let back: EngineSpec =
            serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
        assert_eq!(back.option_pairs(), spec.option_pairs());
    }

    #[test]
    fn bad_options_are_rejected() {
        for options in [
            r#"{"":"1"}"#,
            r#"{"Hash":""}"#,
            r#"{"Ha=sh":"1"}"#,
            r#"{"Hash":"1\n2"}"#,
            r#"{"Ha\nsh":"1"}"#,
            r#"{"Hash":[1]}"#,
            r#"{"Hash":null}"#,
            r#"{"Hash":{"a":1}}"#,
        ] {
            assert!(
                spec_with_options(options).option_pairs().is_err(),
                "{options}"
            );

            let mut spec = valid_spec();
            spec.player2 = spec_with_options(options);
            assert!(validate_job(&spec).is_err(), "{options}");
        }
    }

    #[test]
    fn validate_job_accepts_options() {
        let mut spec = valid_spec();
        spec.player1 = spec_with_options(r#"{"Hash":"128"}"#);
        assert!(validate_job(&spec).is_ok());
    }
}
