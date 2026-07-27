use crate::game::Game;
use crate::parse::parse;
use crate::players::{Protocol, get_protocol};
use crate::sprt::Model;
use crate::tournament::{SPRTSettings, TournamentSettings};
use std::env;
use std::path::Path;
use std::process::ExitCode;

mod elo;
mod game;
mod parse;
mod players;
mod settings_file;
mod sprt;
mod stats;
mod tournament;

const HELP: &str = "\
Usage: cutergames [options]

Options:
  --settings <path>         Load settings from a JSON file (see example.json).
                            Any other options given override the file; a
                            --player, --trinomial or --pentanomial replaces
                            the file's whole value.
  --tc <ms>[+<inc ms>]      Time control for every player: a clock with an
  --st <ms>                 optional increment, a fixed time per move, or a
  --depth <n>               fixed depth. Only one can be given, and a
                            player's own tc, st or depth overrides it.
                            Default st 10. All players must use the same kind
                            of time control.
  --player <key=value ...>  Add a player. Keys: name, path, proto, parameters,
                            tc, st, depth, and the flag 'debug'. 'path' is
                            required, 'proto' is one of ugi/uai/uci/tei. tc, st
                            and depth take the same values as --tc, --st and
                            --depth, and override them for this player.
                            'option.<Name>=
                            <value>' sends 'setoption name <Name> value
                            <value>' to the engine at startup and can be given
                            more than once, e.g. \"option.Hash=128\".
  --game <name>             Game to play: ataxx, chess, connect4, droptaxx,
                            gomoku, isolation, pijersi, or tak (6x6).
                            Default ataxx.
  --fens <path>             File of opening positions, one per line. Blank
                            lines and lines starting with '#' are ignored.
  --games <n>               Number of games to play. Unlimited if omitted.
  --threads <n>             Number of games to run simultaneously. Default 1.
  --updates <n>             Print a results update every n games. Default 10.
  --trinomial <key=value>   Enable a trinomial SPRT. Keys: alpha, beta, elo0,
                            elo1, model, and the flag 'autostop'. 'model' says
                            what elo0 and elo1 mean: normalized (nElo, the
                            default), logistic, or bayesian. Matches fastchess.
  --pentanomial <...>       As --trinomial, using pentanomial statistics. The
                            bayesian model is not available.
  --results <path>          Save the settings and results to a JSON file as
                            the match runs. The file can be loaded with
                            --settings to play the same match again.
  --resume <path>           Continue the unfinished match in a --results file,
                            saving to the same file. Only --threads, --updates
                            and --verbose can be given with it.
  --verbose                 Print detailed progress information.
  --help                    Print this message.
  --version                 Print the version.
  --about                   Print information about CuterGames.

Press Enter at any time while a tournament is running to print the
current standings.

Example:
  cutergames --player name=\"A\" path=./a proto=ugi \\
             --player name=\"B\" path=./b proto=ugi \\
             --fens openings.txt --games 1000 --threads 4";

fn validate_sprt(name: &str, sprt: &SPRTSettings) -> Result<(), String> {
    if sprt.elo0 >= sprt.elo1 {
        return Err(format!("--{name}: elo0 must be less than elo1"));
    }
    if sprt.alpha <= 0.0 || sprt.alpha >= 1.0 {
        return Err(format!("--{name}: alpha must be a number between 0 and 1"));
    }
    if sprt.beta <= 0.0 || sprt.beta >= 1.0 {
        return Err(format!("--{name}: beta must be a number between 0 and 1"));
    }
    if sprt.alpha + sprt.beta >= 1.0 {
        return Err(format!(
            "--{name}: the sum of alpha and beta must be less than 1"
        ));
    }
    if name == "pentanomial" && sprt.model == Model::Bayesian {
        return Err(
            "--pentanomial: the bayesian model is only available for trinomial statistics"
                .to_string(),
        );
    }
    Ok(())
}

fn validate(settings: &TournamentSettings) -> Result<(), String> {
    if settings.players.len() < 2 {
        return Err(format!(
            "At least 2 players are required, got {}",
            settings.players.len()
        ));
    }

    for player in &settings.players {
        match get_protocol(&player.proto) {
            None => {
                return Err(format!(
                    "Player \"{}\" has an unknown protocol \"{}\", expected one of ugi/uai/uci/tei",
                    player.name, player.proto
                ));
            }
            Some(Protocol::TEI) if settings.game != Game::Tak => {
                return Err(format!(
                    "Player \"{}\" uses the tei protocol, which only supports tak",
                    player.name
                ));
            }
            Some(_) => {}
        }
    }

    // The go command assumes both players share one time control kind
    if let Some(first) = settings.players.first()
        && settings.players.iter().any(|p| {
            std::mem::discriminant(&settings.player_clock(p))
                != std::mem::discriminant(&settings.player_clock(first))
        })
    {
        return Err("All players must use the same kind of time control (tc)".to_string());
    }

    for (name, sprt) in [
        ("trinomial", &settings.sprt_trinomial),
        ("pentanomial", &settings.sprt_pentanomial),
    ] {
        if let Some(sprt) = sprt {
            validate_sprt(name, sprt)?;
        }
    }

    if settings.fens.is_empty() {
        return Err("--fens must be set".to_string());
    }

    if settings.num_threads == 0 {
        return Err("--threads must be at least 1".to_string());
    }

    if settings.update_frequency == 0 {
        return Err("--updates must be at least 1".to_string());
    }

    validate_results(settings)?;

    Ok(())
}

// A new match won't overwrite an earlier one's results
fn validate_results(settings: &TournamentSettings) -> Result<(), String> {
    if let Some(path) = &settings.results
        && !settings.resume
        && Path::new(path).exists()
    {
        return Err(format!(
            "The results file \"{}\" already exists. Choose another path, or use --resume to continue an unfinished match",
            path
        ));
    }

    Ok(())
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<String>>();

    if args.contains(&"--help".to_string()) {
        println!("{}", HELP);
        return Ok(());
    } else if args.contains(&"--version".to_string()) {
        println!("CuterGames {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    } else if args.contains(&"--about".to_string()) {
        println!("CuterGames {}", env!("CARGO_PKG_VERSION"));
        println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        println!("{}", env!("CARGO_PKG_HOMEPAGE"));
        return Ok(());
    }

    // Skip the path we were invoked with
    let settings = parse(args.into_iter().skip(1)).map_err(|err| err.to_string())?;

    validate(&settings)?;

    if settings.players.len() > 2 {
        println!(
            "<Warning> {} players given, but only the first 2 are currently used",
            settings.players.len()
        );
    }

    if settings.verbose {
        println!("{:#?}", settings);
    }

    tournament::run::run(&settings)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!("Try --help for a list of options.");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprt() -> SPRTSettings {
        SPRTSettings {
            alpha: 0.05,
            beta: 0.05,
            elo0: 0.0,
            elo1: 2.0,
            autostop: false,
            model: Model::Normalized,
        }
    }

    #[test]
    fn existing_results_file() {
        let existing = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let missing = concat!(env!("CARGO_MANIFEST_DIR"), "/no-such-results.json");

        let settings = |path: &str, resume: bool| TournamentSettings {
            results: Some(path.to_string()),
            resume,
            ..Default::default()
        };

        assert!(validate_results(&TournamentSettings::default()).is_ok());
        assert!(validate_results(&settings(missing, false)).is_ok());
        assert!(validate_results(&settings(existing, false)).is_err());
        assert!(validate_results(&settings(existing, true)).is_ok());
    }

    #[test]
    fn valid_sprt_settings() {
        assert!(validate_sprt("trinomial", &sprt()).is_ok());
        assert!(validate_sprt("pentanomial", &sprt()).is_ok());
    }

    #[test]
    fn invalid_sprt_settings() {
        let bad = [
            SPRTSettings {
                elo0: 2.0,
                ..sprt()
            },
            SPRTSettings {
                elo0: 3.0,
                ..sprt()
            },
            SPRTSettings {
                alpha: 0.0,
                ..sprt()
            },
            SPRTSettings {
                alpha: 1.0,
                ..sprt()
            },
            SPRTSettings {
                beta: 0.0,
                ..sprt()
            },
            SPRTSettings {
                beta: 1.5,
                ..sprt()
            },
            SPRTSettings {
                alpha: 0.5,
                beta: 0.5,
                ..sprt()
            },
        ];
        for settings in bad {
            assert!(
                validate_sprt("trinomial", &settings).is_err(),
                "{settings:?}"
            );
        }
    }

    #[test]
    fn bayesian_is_trinomial_only() {
        let bayesian = SPRTSettings {
            model: Model::Bayesian,
            ..sprt()
        };
        assert!(validate_sprt("trinomial", &bayesian).is_ok());
        assert!(validate_sprt("pentanomial", &bayesian).is_err());
    }
}
