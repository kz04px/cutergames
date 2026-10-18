use protocol::{EngineSpec, GameOutcome, Task, TaskResult};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// `--player key=value ...` args for one engine. The runner rejects a
// present-but-empty value (e.g. "parameters=") as a missing parameter, so
// only include a key when there's actually something to say.
fn player_args(spec: &EngineSpec) -> Vec<String> {
    let mut args = vec![
        "--player".to_string(),
        format!("name={}", spec.name),
        format!("path={}", spec.path),
        format!("proto={}", spec.proto),
        spec.tc.clone(),
    ];
    if !spec.parameters.is_empty() {
        args.push(format!("parameters={}", spec.parameters));
    }
    // The server has already validated the options, so they can be sent
    // as they are; one that somehow isn't valid is left out.
    for (name, value) in spec.option_pairs().unwrap_or_default() {
        args.push(format!("option.{name}={value}"));
    }
    args
}

fn parse_outcome(stdout: &str) -> Option<GameOutcome> {
    let line = stdout.lines().find(|l| l.starts_with("Score of "))?;
    let rest = line.split_once(": ")?.1;
    let mut parts = rest.split_whitespace();
    let w: usize = parts.next()?.parse().ok()?;
    parts.next()?; // "-"
    let l: usize = parts.next()?.parse().ok()?;
    parts.next()?; // "-"
    let d: usize = parts.next()?.parse().ok()?;

    // Exactly one of these is 1 -- a single game was played.
    if w == 1 {
        Some(GameOutcome::P1win)
    } else if l == 1 {
        Some(GameOutcome::P2win)
    } else if d == 1 {
        Some(GameOutcome::Draw)
    } else {
        None
    }
}

fn parse_ply(stdout: &str) -> usize {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("Average game length: "))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|ply| ply.parse::<f32>().ok())
        .map_or(0, |ply| ply.round() as usize)
}

// A single-fen openings file for the runner's --fens, unique per task so
// concurrent tasks (or a leftover file from a crashed run) never collide.
struct FenFile(PathBuf);

impl FenFile {
    fn new(fen: &str) -> std::io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!("cutergames-client-{pid}-{now}-{unique}.fen"));
        fs::write(&path, format!("{fen}\n"))?;
        Ok(Self(path))
    }
}

impl Drop for FenFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

// Plays one task by invoking the runner as a subprocess and parsing its
// final "Score of ..." / "Average game length: ..." lines.
pub fn run_task(runner_path: &str, task: &Task) -> TaskResult {
    let no_result = TaskResult {
        job_id: task.job_id,
        game_id: task.game_id,
        outcome: None,
        ply: 0,
    };

    let fen_file = match FenFile::new(&task.fen) {
        Ok(f) => f,
        Err(err) => {
            println!("Failed to write a temporary fens file: {}", err);
            return no_result;
        }
    };

    let mut command = Command::new(runner_path);
    command
        .args(player_args(&task.player1))
        .args(player_args(&task.player2))
        .arg("--game")
        .arg(&task.game)
        .arg("--fens")
        .arg(&fen_file.0)
        .arg("--games")
        .arg("1");

    let output = match command.output() {
        Ok(output) => output,
        Err(err) => {
            println!("Failed to run \"{}\": {}", runner_path, err);
            return no_result;
        }
    };

    if !output.status.success() {
        println!(
            "\"{}\" exited with {}: {}",
            runner_path,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
        return no_result;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let outcome = parse_outcome(&stdout);
    if outcome.is_none() {
        println!(
            "Couldn't find a game result in the runner's output for job #{} game #{}",
            task.job_id, task.game_id
        );
    }

    TaskResult {
        job_id: task.job_id,
        game_id: task.game_id,
        outcome,
        ply: parse_ply(&stdout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_player1_win() {
        let stdout = "Score of A vs B: 1 - 0 - 0 [1.000] 1\n\nMatch Statistics:\n";
        assert_eq!(parse_outcome(stdout), Some(GameOutcome::P1win));
    }

    #[test]
    fn parses_player2_win() {
        let stdout = "Score of A vs B: 0 - 1 - 0 [0.000] 1\n";
        assert_eq!(parse_outcome(stdout), Some(GameOutcome::P2win));
    }

    #[test]
    fn parses_draw() {
        let stdout = "Score of A vs B: 0 - 0 - 1 [0.500] 1\n";
        assert_eq!(parse_outcome(stdout), Some(GameOutcome::Draw));
    }

    #[test]
    fn missing_score_line_is_none() {
        assert_eq!(parse_outcome("nothing useful here\n"), None);
    }

    #[test]
    fn parses_ply() {
        let stdout = "Games played: 1\nAverage game length: 42.0 ply\n";
        assert_eq!(parse_ply(stdout), 42);
    }

    #[test]
    fn missing_ply_defaults_to_zero() {
        assert_eq!(parse_ply("nothing useful here\n"), 0);
    }

    #[test]
    fn parameters_omitted_when_empty() {
        let spec = EngineSpec {
            name: "A".to_string(),
            path: "./a".to_string(),
            proto: "ugi".to_string(),
            parameters: String::new(),
            tc: "st=10".to_string(),
            options: Default::default(),
        };
        assert!(
            !player_args(&spec)
                .iter()
                .any(|a| a.starts_with("parameters="))
        );
    }

    #[test]
    fn tc_passed_through() {
        let spec = EngineSpec {
            name: "A".to_string(),
            path: "./a".to_string(),
            proto: "ugi".to_string(),
            parameters: String::new(),
            tc: "tc=1000+10".to_string(),
            options: Default::default(),
        };
        assert!(player_args(&spec).iter().any(|a| a == "tc=1000+10"));
    }

    #[test]
    fn parameters_included_when_set() {
        let spec = EngineSpec {
            name: "A".to_string(),
            path: "./a".to_string(),
            proto: "ugi".to_string(),
            parameters: "--foo".to_string(),
            tc: "st=10".to_string(),
            options: Default::default(),
        };
        assert!(player_args(&spec).iter().any(|a| a == "parameters=--foo"));
    }

    #[test]
    fn options_become_option_arguments_in_order() {
        let spec: EngineSpec = serde_json::from_str(
            r#"{"name":"A","path":"./a","proto":"ugi","options":{"Hash":"128","Skill Level":5}}"#,
        )
        .unwrap();
        let args = player_args(&spec);
        let options: Vec<&String> = args.iter().filter(|a| a.starts_with("option.")).collect();
        assert_eq!(options, ["option.Hash=128", "option.Skill Level=5"]);
    }

    #[test]
    fn no_option_arguments_without_options() {
        let spec: EngineSpec =
            serde_json::from_str(r#"{"name":"A","path":"./a","proto":"ugi"}"#).unwrap();
        assert!(!player_args(&spec).iter().any(|a| a.starts_with("option.")));
    }
}
