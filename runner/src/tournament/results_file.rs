use crate::{
    elo::{pentanomial, trinomial},
    settings_file::{f32_value, to_json},
    sprt,
    stats::{WLD, WLDPairs},
    tournament::{
        PlayerStatistics, SPRTSettings, TournamentSettings, TournamentStatistics, play::GameOutcome,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Running,
    // Ended before the game limit or an SPRT bound, so it can be resumed
    Stopped,
    Complete,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Complete => "complete",
        }
    }
}

// Whether the match has reached its game limit or an SPRT bound that stops
// it, rather than having been stopped part way
pub fn is_complete(
    settings: &TournamentSettings,
    player_stats: &HashMap<usize, PlayerStatistics>,
    next_game: usize,
    unplayed: &[usize],
) -> bool {
    let wld = &player_stats[&0].wld;
    let pairs = &player_stats[&0].wld_pairs;
    let sprt_stop = |sprt: &Option<SPRTSettings>, llr: &dyn Fn(&SPRTSettings) -> f32| {
        sprt.as_ref()
            .is_some_and(|s| s.autostop && sprt::should_stop(llr(s), s.alpha, s.beta))
    };

    sprt_stop(&settings.sprt_trinomial, &|s| {
        sprt::trinomial::get_llr(wld, s.elo0, s.elo1, s.model)
    }) || sprt_stop(&settings.sprt_pentanomial, &|s| {
        sprt::pentanomial::get_llr(pairs, s.elo0, s.elo1, s.model)
    }) || settings
        .num_games
        .is_some_and(|num| next_game >= num && unplayed.is_empty())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResumePlayer {
    pub played: usize,
    pub crashes: usize,
    pub timeouts: usize,
    pub illegal_moves: usize,
    pub wld: WLD,
    pub pairs: WLDPairs,
}

// Everything a resumed match continues from, beyond the settings. Only saved
// while the match is unfinished.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResumeState {
    // The next game the generator would have handed out
    pub next_game: usize,
    // Games before next_game without a result, either still being played or
    // ended without one
    pub unplayed_games: Vec<usize>,
    // Pairs with only one game played, by that game's id
    pub half_pairs: BTreeMap<usize, GameOutcome>,
    pub games: usize,
    pub total_ply: usize,
    pub players: Vec<ResumePlayer>,
}

impl ResumeState {
    pub fn capture(
        tournament_stats: &TournamentStatistics,
        player_stats: &HashMap<usize, PlayerStatistics>,
        pair_store: &HashMap<usize, GameOutcome>,
        next_game: usize,
        unplayed_games: Vec<usize>,
    ) -> Self {
        Self {
            next_game,
            unplayed_games,
            half_pairs: pair_store.iter().map(|(&id, &o)| (id, o)).collect(),
            games: tournament_stats.games_completed,
            total_ply: tournament_stats.total_ply,
            players: (0..player_stats.len())
                .map(|idx| {
                    let stats = &player_stats[&idx];
                    ResumePlayer {
                        played: stats.played,
                        crashes: stats.crashes,
                        timeouts: stats.timeouts,
                        illegal_moves: stats.illegalmoves,
                        wld: WLD {
                            w: stats.wld.w,
                            l: stats.wld.l,
                            d: stats.wld.d,
                        },
                        pairs: WLDPairs {
                            ww: stats.wld_pairs.ww,
                            wl: stats.wld_pairs.wl,
                            wd: stats.wld_pairs.wd,
                            dw: stats.wld_pairs.dw,
                            dl: stats.wld_pairs.dl,
                            dd: stats.wld_pairs.dd,
                            lw: stats.wld_pairs.lw,
                            ll: stats.wld_pairs.ll,
                            ld: stats.wld_pairs.ld,
                        },
                    }
                })
                .collect(),
        }
    }

    pub fn restore(
        self,
        tournament_stats: &mut TournamentStatistics,
        player_stats: &mut HashMap<usize, PlayerStatistics>,
        pair_store: &mut HashMap<usize, GameOutcome>,
    ) {
        tournament_stats.games_completed = self.games;
        tournament_stats.total_ply = self.total_ply;
        pair_store.extend(self.half_pairs);
        for (idx, saved) in self.players.into_iter().enumerate() {
            let stats = player_stats
                .get_mut(&idx)
                .expect("player_stats has an entry for every player");
            stats.played = saved.played;
            stats.crashes = saved.crashes;
            stats.timeouts = saved.timeouts;
            stats.illegalmoves = saved.illegal_moves;
            stats.wld = saved.wld;
            stats.wld_pairs = saved.pairs;
        }
    }
}

// The settings a match ran with, as a settings file, plus a "results" block
// that the settings loader skips. Loading the file with --settings plays the
// same match again, and the hashes show whether the engines or openings have
// changed since. --resume continues it if it's unfinished.
pub struct ResultsFile {
    path: String,
    started: String,
    // When the match was continued with --resume
    resumed: Vec<String>,
    // Taken when the match starts, so they describe the files actually used
    engine_hashes: Vec<Option<String>>,
    openings_hash: Option<String>,
}

#[derive(Deserialize)]
struct SavedPlayer {
    sha256: Option<String>,
}

// The parts of a saved results block that a resumed match needs
#[derive(Deserialize)]
struct SavedResults {
    started: String,
    #[serde(default)]
    resumed: Vec<String>,
    players: Vec<SavedPlayer>,
    openings_sha256: Option<String>,
    resume: Option<ResumeState>,
}

impl ResultsFile {
    pub fn new(path: &str, settings: &TournamentSettings) -> Self {
        Self {
            path: path.to_string(),
            started: utc_now(),
            resumed: vec![],
            engine_hashes: settings
                .players
                .iter()
                .map(|p| sha256_file(&p.path))
                .collect(),
            openings_hash: settings.openings.as_deref().and_then(sha256_file),
        }
    }

    // The unfinished match saved at `path`, whose settings were loaded into
    // `settings`
    pub fn resume(
        path: &str,
        settings: &TournamentSettings,
    ) -> Result<(Self, ResumeState), String> {
        let text = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read the results file \"{}\": {}", path, err))?;
        let mut json: Value = serde_json::from_str(&text)
            .map_err(|err| format!("Invalid results file \"{}\": {}", path, err))?;
        let saved: SavedResults = serde_json::from_value(json["results"].take())
            .map_err(|err| format!("Invalid results in \"{}\": {}", path, err))?;

        let Some(state) = saved.resume else {
            return Err(format!(
                "The match in \"{}\" is complete, so there's nothing to resume",
                path
            ));
        };
        if state.players.len() != settings.players.len()
            || saved.players.len() != settings.players.len()
        {
            return Err(format!(
                "Invalid results in \"{}\": the results don't match the players",
                path
            ));
        }

        if state
            .unplayed_games
            .iter()
            .chain(state.half_pairs.keys())
            .any(|&id| id >= state.next_game)
        {
            return Err(format!(
                "Invalid results in \"{}\": a game is past \"next_game\"",
                path
            ));
        }

        let mut file = Self::new(path, settings);

        // Combining results from different engines or openings would
        // describe neither
        for (idx, saved_player) in saved.players.iter().enumerate() {
            if saved_player.sha256 != file.engine_hashes[idx] {
                return Err(format!(
                    "The engine \"{}\" has changed since the match started, so it can't be resumed",
                    settings.players[idx].path
                ));
            }
        }
        if saved.openings_sha256 != file.openings_hash {
            return Err(
                "The openings file has changed since the match started, so it can't be resumed"
                    .to_string(),
            );
        }

        file.started = saved.started;
        file.resumed = saved.resumed;
        file.resumed.push(utc_now());

        Ok((file, state))
    }

    pub fn write(
        &self,
        settings: &TournamentSettings,
        tournament_stats: &TournamentStatistics,
        player_stats: &HashMap<usize, PlayerStatistics>,
        status: Status,
        resume: &ResumeState,
    ) -> Result<(), String> {
        let mut out = to_json(settings);
        out["results"] = self.results(settings, tournament_stats, player_stats, status);
        if status != Status::Complete {
            out["results"]["resume"] =
                serde_json::to_value(resume).expect("ResumeState always serialises");
        }

        let text = serde_json::to_string_pretty(&out).expect("JSON values always serialise");

        // Written alongside and renamed into place, so a match that's killed
        // mid-write still leaves the previous complete file behind
        let tmp = format!("{}.tmp", self.path);
        fs::write(&tmp, text + "\n")
            .and_then(|()| fs::rename(&tmp, &self.path))
            .map_err(|err| {
                format!(
                    "Failed to write the results file \"{}\": {}",
                    self.path, err
                )
            })
    }

    fn results(
        &self,
        settings: &TournamentSettings,
        tournament_stats: &TournamentStatistics,
        player_stats: &HashMap<usize, PlayerStatistics>,
        status: Status,
    ) -> Value {
        let mut out = Map::new();
        out.insert("cutergames".to_string(), version().into());
        out.insert("status".to_string(), status.name().into());
        out.insert("started".to_string(), self.started.clone().into());
        if !self.resumed.is_empty() {
            out.insert("resumed".to_string(), self.resumed.clone().into());
        }
        out.insert(
            "finished".to_string(),
            if status == Status::Running {
                Value::Null
            } else {
                utc_now().into()
            },
        );
        out.insert("games".to_string(), tournament_stats.games_completed.into());
        if tournament_stats.games_completed > 0 {
            out.insert(
                "average_ply".to_string(),
                f32_value(
                    tournament_stats.total_ply as f32 / tournament_stats.games_completed as f32,
                ),
            );
        }

        out.insert(
            "players".to_string(),
            Value::Array(
                (0..settings.players.len())
                    .map(|idx| {
                        let stats = &player_stats[&idx];
                        json!({
                            "name": stats.name,
                            "sha256": self.engine_hashes[idx],
                            "crashes": stats.crashes,
                            "timeouts": stats.timeouts,
                            "illegal_moves": stats.illegalmoves,
                        })
                    })
                    .collect(),
            ),
        );
        out.insert(
            "openings_sha256".to_string(),
            self.openings_hash.clone().into(),
        );

        // Only the first two players play, and everything below is from the
        // first player's point of view
        let wld = &player_stats[&0].wld;
        let pairs = &player_stats[&0].wld_pairs;
        out.insert("wld".to_string(), json!([wld.w, wld.l, wld.d]));
        out.insert("pairs".to_string(), json!(pair_buckets(pairs)));

        let (elo, elo_err) = trinomial::elo_err(wld);
        let (nelo, nelo_err) = trinomial::nelo_err(wld);
        out.insert(
            "trinomial".to_string(),
            elo_json(elo, elo_err, nelo, nelo_err),
        );
        let (elo, elo_err) = pentanomial::elo_err(pairs);
        let (nelo, nelo_err) = pentanomial::nelo_err(pairs);
        out.insert(
            "pentanomial".to_string(),
            elo_json(elo, elo_err, nelo, nelo_err),
        );

        if let Some(tri) = &settings.sprt_trinomial {
            let llr = sprt::trinomial::get_llr(wld, tri.elo0, tri.elo1, tri.model);
            out["trinomial"]["sprt"] = sprt_json(llr, tri);
        }
        if let Some(penta) = &settings.sprt_pentanomial {
            let llr = sprt::pentanomial::get_llr(pairs, penta.elo0, penta.elo1, penta.model);
            out["pentanomial"]["sprt"] = sprt_json(llr, penta);
        }

        Value::Object(out)
    }
}

// Pairs scored +2, +1, 0, -1 and -2
fn pair_buckets(pairs: &WLDPairs) -> [usize; 5] {
    [
        pairs.ww,
        pairs.wd + pairs.dw,
        pairs.wl + pairs.lw + pairs.dd,
        pairs.ld + pairs.dl,
        pairs.ll,
    ]
}

fn elo_json(elo: f32, elo_err: f32, nelo: f32, nelo_err: f32) -> Value {
    json!({
        "elo": f32_value(elo),
        "elo_err": f32_value(elo_err),
        "nelo": f32_value(nelo),
        "nelo_err": f32_value(nelo_err),
    })
}

fn sprt_json(llr: f32, settings: &SPRTSettings) -> Value {
    let lbound = sprt::get_lbound(settings.alpha, settings.beta);
    let ubound = sprt::get_ubound(settings.alpha, settings.beta);
    let result = if llr >= ubound {
        Value::from("H1")
    } else if llr <= lbound {
        Value::from("H0")
    } else {
        Value::Null
    };

    json!({
        "llr": f32_value(llr),
        "lbound": f32_value(lbound),
        "ubound": f32_value(ubound),
        "result": result,
    })
}

fn version() -> String {
    match option_env!("CUTERGAMES_COMMIT") {
        Some(commit) => format!("{} ({})", env!("CARGO_PKG_VERSION"), commit),
        None => env!("CARGO_PKG_VERSION").to_string(),
    }
}

// None if the file can't be read, e.g. an engine started by name from PATH
fn sha256_file(path: &str) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect(),
    )
}

fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format_utc(secs)
}

// Seconds since the Unix epoch as an ISO 8601 UTC time,
// e.g. 2026-10-18T16:02:11Z
fn format_utc(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;

    // Civil date from days since 1970-01-01, after Howard Hinnant's
    // days_from_civil inverse
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(w: usize, l: usize, d: usize) -> PlayerStatistics {
        PlayerStatistics {
            name: "A".to_string(),
            played: w + l + d,
            crashes: 1,
            timeouts: 2,
            illegalmoves: 3,
            wld: WLD { w, l, d },
            wld_pairs: WLDPairs {
                ww: 1,
                dd: 2,
                lw: 3,
                ..Default::default()
            },
        }
    }

    #[test]
    fn resume_state_round_trips() {
        let player_stats = HashMap::from([(0, stats(5, 3, 2)), (1, stats(3, 5, 2))]);
        let tournament_stats = TournamentStatistics {
            games_completed: 10,
            total_ply: 400,
            ..Default::default()
        };
        let pair_store = HashMap::from([(10, GameOutcome::P1win), (13, GameOutcome::Draw)]);

        let state = ResumeState::capture(
            &tournament_stats,
            &player_stats,
            &pair_store,
            14,
            vec![11, 12],
        );
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["half_pairs"]["10"], "p1win");
        let state: ResumeState = serde_json::from_value(json).unwrap();

        let mut restored_tournament = TournamentStatistics::default();
        let mut restored_players = HashMap::from([(0, stats(0, 0, 0)), (1, stats(0, 0, 0))]);
        let mut restored_pairs = HashMap::new();
        state.restore(
            &mut restored_tournament,
            &mut restored_players,
            &mut restored_pairs,
        );

        assert_eq!(restored_tournament.games_completed, 10);
        assert_eq!(restored_tournament.total_ply, 400);
        assert_eq!(restored_pairs, pair_store);
        for idx in 0..2 {
            let (a, b) = (&restored_players[&idx], &player_stats[&idx]);
            assert_eq!(a.played, b.played);
            assert_eq!(a.wld, b.wld);
            assert_eq!(a.wld_pairs, b.wld_pairs);
            assert_eq!(
                (a.crashes, a.timeouts, a.illegalmoves),
                (b.crashes, b.timeouts, b.illegalmoves)
            );
        }
    }

    #[test]
    fn completion() {
        let player_stats = HashMap::from([(0, stats(1, 1, 1)), (1, stats(1, 1, 1))]);
        let mut settings = TournamentSettings {
            num_games: Some(10),
            ..Default::default()
        };
        assert!(is_complete(&settings, &player_stats, 10, &[]));
        assert!(!is_complete(&settings, &player_stats, 10, &[4]));
        assert!(!is_complete(&settings, &player_stats, 8, &[]));

        // Unlimited, so only an SPRT bound can complete it
        settings.num_games = None;
        assert!(!is_complete(&settings, &player_stats, 100, &[]));

        let winning = HashMap::from([(0, stats(500, 0, 0)), (1, stats(0, 500, 0))]);
        settings.sprt_trinomial = Some(SPRTSettings {
            alpha: 0.05,
            beta: 0.05,
            elo0: 0.0,
            elo1: 10.0,
            autostop: false,
            model: Default::default(),
        });
        assert!(!is_complete(&settings, &winning, 1000, &[]));
        settings.sprt_trinomial.as_mut().unwrap().autostop = true;
        assert!(is_complete(&settings, &winning, 1000, &[]));
    }

    #[test]
    fn utc_times() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_792_339_331), "2026-10-18T16:02:11Z");
    }

    #[test]
    fn sprt_results() {
        let settings = SPRTSettings {
            alpha: 0.05,
            beta: 0.05,
            ..Default::default()
        };
        assert_eq!(sprt_json(3.0, &settings)["result"], "H1");
        assert_eq!(sprt_json(-3.0, &settings)["result"], "H0");
        assert_eq!(sprt_json(0.5, &settings)["result"], Value::Null);
    }

    #[test]
    fn pair_buckets_add_up() {
        let pairs = WLDPairs {
            ww: 1,
            wd: 2,
            dw: 3,
            wl: 4,
            lw: 5,
            dd: 6,
            ld: 7,
            dl: 8,
            ll: 9,
        };
        assert_eq!(pair_buckets(&pairs), [1, 5, 15, 15, 9]);
    }
}
