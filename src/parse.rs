use crate::tournament::{PlayerSettings, TournamentSettings};
use std::{borrow::Borrow, env};

#[derive(PartialEq, Debug)]
pub enum ParseError {
    Test(String),
}

#[must_use]
pub fn parse_commandline() -> Result<TournamentSettings, ParseError> {
    parse(&env::args().collect::<Vec<String>>())
}

#[must_use]
fn parse<T>(words: &Vec<T>) -> Result<TournamentSettings, ParseError>
where
    T: Borrow<str>,
{
    let mut settings = TournamentSettings::default();

    // Parser state
    let mut is_player = false;
    let mut is_games = false;
    let mut is_threads = false;
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
            word => {
                if is_games {
                    settings.num_games = word.parse::<usize>().ok();
                    is_games = false;
                } else if is_threads {
                    settings.num_threads = word.parse::<usize>().unwrap();
                    is_threads = false;
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

    if settings.players.len() < 2 {
        return Err(ParseError::Test("Must be at least two players".to_string()));
    } else if settings.num_games.is_none() {
        return Err(ParseError::Test(
            "At least one game must be played".to_string(),
        ));
    } else if settings.num_threads < 1 {
        return Err(ParseError::Test(
            "At least one thread is required".to_string(),
        ));
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
                    },
                    PlayerSettings {
                        name: "Some name 2".to_string(),
                        path: "test2".to_string(),
                        proto: "test".to_string(),
                        parameters: "--one --two two".to_string(),
                    }
                ],
                fens: vec![],
                num_threads: 4,
                num_games: Some(123),
                sprt_trinomial: false,
                sprt_pentanomial: false,
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
