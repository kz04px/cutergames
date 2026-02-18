use crate::tournament::{PlayerSettings, TournamentSettings};
use std::{borrow::Borrow, fs::read_to_string};

#[derive(PartialEq, Debug)]
pub enum ParseError {
    Test(String),
}

pub fn parse<T>(words: &Vec<T>) -> Result<TournamentSettings, ParseError>
where
    T: Borrow<str>,
{
    let mut settings = TournamentSettings::default();

    // Parser state
    let mut is_player = false;
    let mut is_games = false;
    let mut is_threads = false;
    let mut is_fens = false;
    let mut is_elo = false;
    let mut player = PlayerSettings::default();

    for word in words {
        if is_player as i32 + is_games as i32 + is_threads as i32 > 1 {
            return Err(ParseError::Test("Multiple flags true at once".to_string()));
        }

        match word.borrow() {
            "--player" => {
                if is_player {
                    settings.players.push(player.clone());
                }
                player = PlayerSettings::default();
                is_player = true;
            }
            "--games" => {
                if is_player {
                    settings.players.push(player.clone());
                    is_player = false;
                }
                is_games = true
            }
            "--threads" => {
                if is_player {
                    settings.players.push(player.clone());
                    is_player = false;
                }
                is_threads = true
            }
            "--verbose" => {
                if is_player {
                    settings.players.push(player.clone());
                    is_player = false;
                }
                settings.verbose = true;
            }
            "--fens" => {
                if is_player {
                    settings.players.push(player.clone());
                    is_player = false;
                }
                is_fens = true;
            }
            "--elo" => {
                if is_player {
                    settings.players.push(player.clone());
                    is_player = false;
                }
                is_elo = true;
            }
            word => {
                if is_games {
                    settings.num_games = word.parse::<usize>().ok();
                    is_games = false;
                } else if is_threads {
                    settings.num_threads = word.parse::<usize>().unwrap();
                    is_threads = false;
                } else if is_fens {
                    if let Ok(file) = read_to_string(word) {
                        settings.fens = file.lines().map(String::from).collect();
                    } else {
                        return Err(ParseError::Test("Fen file not found".to_string()));
                    }
                    is_fens = false;
                } else if is_elo {
                    match word {
                        "tri" => settings.sprt_trinomial = true,
                        "penta" => settings.sprt_pentanomial = true,
                        _ => {}
                    }
                    is_elo = false;
                } else if let Some((first, second)) = word.split_once('=') {
                    if !is_player {
                        return Err(ParseError::Test(
                            "Parsing player arguments without player flag".to_string(),
                        ));
                    }
                    match first {
                        "name" => player.name = second.to_string(),
                        "path" => player.path = second.to_string(),
                        "proto" => player.proto = second.to_string(),
                        "parameters" => {
                            if player.parameters.is_empty() {
                                player.parameters = second.to_string();
                            } else {
                                player.parameters += " ";
                                player.parameters += second;
                            }
                        }
                        _ => {
                            return Err(ParseError::Test("Unrecognised player flag".to_string()));
                        }
                    }
                } else if is_player && word == "debug" {
                    player.debug = true
                }
            }
        }
    }

    if is_player {
        settings.players.push(player.clone());
    }

    for player in &settings.players {
        if player.name.is_empty() {
            return Err(ParseError::Test("Player name not specified".to_string()));
        } else if player.path.is_empty() {
            return Err(ParseError::Test("Player path not specified".to_string()));
        }
    }

    if settings.fens.is_empty() {
        Err(ParseError::Test("No opening fens loaded".to_string()))
    } else if settings.players.len() < 2 {
        Err(ParseError::Test("Must be at least two players".to_string()))
    } else if settings.num_games.is_none() {
        Err(ParseError::Test(
            "At least one game must be played".to_string(),
        ))
    } else if settings.num_threads < 1 {
        Err(ParseError::Test(
            "At least one thread is required".to_string(),
        ))
    } else {
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        assert_eq!(
            parse(&vec![
                "--player",
                "name=Some name 1",
                "path=test1",
                "parameters=Some thing long here",
                "proto=test",
                "--player",
                "name=Some name 2",
                "path=test2",
                "parameters=--one",
                "parameters=--two two",
                "proto=test",
                "--games",
                "123",
                "--threads",
                "4",
                "--verbose",
            ]),
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
                sprt_trinomial: true,
                sprt_pentanomial: false,
                alpha: 0.05,
                beta: 0.05,
                verbose: true,
            })
        );
    }

    #[test]
    fn failure() {
        assert!(
            parse(&vec![
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
            ])
            .is_err()
        );
    }
}
