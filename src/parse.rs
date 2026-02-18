use crate::tournament::{PlayerSettings, SPRTSettings, TournamentSettings};
use std::{borrow::Borrow, env, fs::read_to_string, iter::Peekable, path::Path};

#[derive(PartialEq, Debug)]
pub enum ParseError {
    ValueParse(String),
    FileParse,
    MissingParameter,
    UnknownParameter,
}

pub fn parse<T>(iter: impl Iterator<Item = T>) -> Result<TournamentSettings, ParseError>
where
    T: Borrow<str> + AsRef<Path>,
{
    let mut settings = TournamentSettings::default();
    let mut iter = iter.peekable();

    while let Some(word) = iter.next() {
        // Flags
        match word.borrow() {
            "--verbose" => settings.verbose = true,
            // "--debug" => settings.debug = true,
            _ => {}
        }

        // Singles
        match (word.borrow(), iter.peek()) {
            ("--games", Some(next)) => {
                if let Ok(value) = next.borrow().parse::<usize>() {
                    settings.num_games = Some(value);
                } else {
                    return Err(ParseError::ValueParse(format!(
                        "Failed to parse usize: {}",
                        next.borrow()
                    )));
                }
            }
            ("--threads", Some(next)) => {
                if let Ok(value) = next.borrow().parse::<usize>() {
                    settings.num_threads = value;
                } else {
                    return Err(ParseError::ValueParse(format!(
                        "Failed to parse usize: {}",
                        next.borrow()
                    )));
                }
            }
            ("--updates", Some(next)) => {
                if let Ok(value) = next.borrow().parse::<usize>() {
                    settings.update_frequency = value;
                } else {
                    return Err(ParseError::ValueParse(format!(
                        "Failed to parse usize: {}",
                        next.borrow()
                    )));
                }
            }
            ("--fens", Some(next)) => {
                if let Ok(file) = read_to_string(next) {
                    settings.fens = file.lines().map(String::from).collect();
                } else {
                    return Err(ParseError::FileParse);
                }
            }
            _ => {}
        }

        // Multiples
        match word.borrow() {
            "--player" => {
                let mut player = PlayerSettings::default();
                while let Some(asd) = iter.peek() {
                    match asd.borrow().split_once('=') {
                        Some((_, "")) => return Err(ParseError::MissingParameter),
                        Some(("name", second)) => player.name = second.to_string(),
                        Some(("path", second)) => player.path = second.to_string(),
                        Some(("proto", second)) => player.proto = second.to_string(),
                        Some(("parameters", second)) => player.parameters = second.to_string(),
                        Some((_, _)) => return Err(ParseError::UnknownParameter),
                        _ => break,
                    }

                    iter.next();
                }
                if player.path.is_empty() {
                    return Err(ParseError::MissingParameter);
                }
                settings.players.push(player);
            }
            "--trinomial" => {
                let mut tri = SPRTSettings::default();
                while let Some(asd) = iter.peek() {
                    // Singles
                    match asd.borrow() {
                        "autostop" => {
                            tri.autostop = true;
                            iter.next();
                            continue;
                        }
                        _ =>
                        // Doubles
                        {
                            match asd.borrow().split_once('=') {
                                Some(("alpha", second)) => {
                                    tri.alpha = second.parse::<f32>().unwrap()
                                }
                                Some(("beta", second)) => tri.beta = second.parse::<f32>().unwrap(),
                                Some(("elo0", second)) => tri.elo0 = second.parse::<f32>().unwrap(),
                                Some(("elo1", second)) => tri.elo1 = second.parse::<f32>().unwrap(),
                                _ => break,
                            }
                            iter.next();
                        }
                    }
                }
                settings.sprt_trinomial = Some(tri);
            }
            _ => {}
        }
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                        debug: false,
                    },
                    PlayerSettings {
                        name: "Some name 2".to_string(),
                        path: "test2".to_string(),
                        proto: "test".to_string(),
                        parameters: "--one --two two".to_string(),
                        debug: false,
                    }
                ],
                fens: vec![],
                num_threads: 4,
                num_games: Some(123),
                update_frequency: 10,
                sprt_trinomial: None,
                sprt_pentanomial: None,
                verbose: true,
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
}
