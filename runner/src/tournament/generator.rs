use std::collections::VecDeque;

enum Pairing {
    RoundRobin,
}

pub struct Generator {
    pairing: Pairing,
    // Every pair of players, lower index first
    matchups: Vec<(usize, usize)>,
    pub id: usize,
    pub num_players: usize,
    pub num_openings: usize,
    pub num_games: Option<usize>,
    // Games before `id` that a resumed match still has to play, handed out
    // first
    replay: VecDeque<usize>,
}

pub struct Work {
    pub game_id: usize,
    pub player1: usize,
    pub player2: usize,
    pub fen_idx: usize,
}

impl Generator {
    #[must_use]
    pub fn new(num_players: usize, num_openings: usize, num_games: Option<usize>) -> Self {
        let matchups = (0..num_players)
            .flat_map(|a| (a + 1..num_players).map(move |b| (a, b)))
            .collect();
        Self {
            pairing: Pairing::RoundRobin,
            matchups,
            id: 0,
            num_players,
            num_openings,
            num_games,
            replay: VecDeque::new(),
        }
    }

    // Continues from `next_id`, after playing `replay`
    #[must_use]
    pub fn resume(mut self, next_id: usize, replay: Vec<usize>) -> Self {
        self.id = next_id;
        self.replay = replay.into();
        self
    }

    #[must_use]
    pub fn next(&mut self) -> Option<Work> {
        // Nothing to play from
        if self.num_openings == 0 || self.num_players < 2 {
            return None;
        }

        if let Some(id) = self.replay.pop_front() {
            return Some(self.work(id));
        }

        // Number of games limit reached
        if let Some(max) = self.num_games
            && self.id >= max
        {
            return None;
        }

        let work = self.work(self.id);
        self.id += 1;

        Some(work)
    }

    // A game's players and opening depend only on its id, so a replayed game
    // is the same game. Games 2k and 2k + 1 are a pair: the same matchup and
    // opening with the colours swapped. Each round plays every matchup once,
    // all on that round's opening.
    fn work(&self, id: usize) -> Work {
        match self.pairing {
            Pairing::RoundRobin => {
                let pair = id / 2;
                let (a, b) = self.matchups[pair % self.matchups.len()];
                let (player1, player2) = if id % 2 == 0 { (a, b) } else { (b, a) };
                Work {
                    game_id: id,
                    player1,
                    player2,
                    fen_idx: (pair / self.matchups.len()) % self.num_openings,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_replays_then_continues() {
        let mut generator = Generator::new(2, 3, Some(8)).resume(5, vec![1, 4]);
        let ids: Vec<_> = std::iter::from_fn(|| generator.next())
            .map(|w| (w.game_id, w.player1, w.fen_idx))
            .collect();
        assert_eq!(ids, [(1, 1, 0), (4, 0, 2), (5, 1, 2), (6, 0, 0), (7, 1, 0)]);
    }

    #[test]
    fn round_robin_plays_every_matchup_with_swapped_colours() {
        let mut generator = Generator::new(3, 2, Some(14));
        let games: Vec<_> = std::iter::from_fn(|| generator.next())
            .map(|w| (w.player1, w.player2, w.fen_idx))
            .collect();
        assert_eq!(
            games,
            [
                (0, 1, 0),
                (1, 0, 0),
                (0, 2, 0),
                (2, 0, 0),
                (1, 2, 0),
                (2, 1, 0),
                (0, 1, 1),
                (1, 0, 1),
                (0, 2, 1),
                (2, 0, 1),
                (1, 2, 1),
                (2, 1, 1),
                (0, 1, 0),
                (1, 0, 0),
            ]
        );
    }
}
